use rustls_connector::{RustlsConnector, TlsStream};
use std::net::TcpStream;
use imap;
use lazy_static::lazy_static;
use log::{error, debug};
use std::ops::Deref;
use std::iter::FromIterator;
use schedule;
use tokio;
use rustyknife::rfc2047::encoded_word;
use std::sync::Arc;

use crate::types::{Result, Error, MailCheckerError};
use crate::storage::{Storage, MailAccount};
use crate::cfg::CONFIG;
use crate::bot::TelegramBot;


lazy_static!{
    static ref STORAGE: Storage = Storage::new().unwrap();
}

pub struct Checker {
    host: String,
    port: u16
}

impl Checker {
    fn new() -> Result<Checker> {
        let host = CONFIG.get::<String>("mail.address");
        let port = CONFIG.get::<u16>("mail.port");

        Ok(
            Checker{
                host,
                port
            }
        )
    }

    fn build_stream(&self) -> Result<TlsStream<TcpStream>> {
        let connector = RustlsConnector::new_with_native_certs()?;
        let stream = TcpStream::connect((self.host.clone(), self.port))?;
        let tls_stream = connector.connect(&self.host, stream)?;
        Ok(tls_stream)
    }

    fn build_client(&self) -> Result<imap::Client<TlsStream<TcpStream>>> {
        let stream = self.build_stream()?;
        Ok(imap::Client::new(stream))
    }

    fn decode_value(data: Option<&[u8]>) -> Option<String> {
        if let Some(data) = data {
            let value = String::from_utf8_lossy(data).into_owned();
            let data_owner = value.clone();
            let data = data_owner.as_bytes();
            let (_, value) = encoded_word(data).unwrap_or((&[], value));
            return Some(value);
        }
        None
    }

    fn process_message(
        message: &imap::types::Fetch,
        bot: &TelegramBot,
        username: &String,
    ) {
        let envelope = message.envelope();
        if let None = envelope {
            let error = Error::MailCheckerError(MailCheckerError::EmptyEnvelope);
            error!("{}", error);
            return;
        }
        let envelope = envelope.unwrap();

        let mut from_addr: Option<&[u8]> = None;
        let mut host: Option<&[u8]> = None;
        let mut mailbox: Option<&[u8]> = None;

        if let Some(addresses) = &envelope.from.as_ref() {
            if addresses.len() > 0 {
                from_addr = addresses[0].name;
                host = addresses[0].host;
                mailbox = addresses[0].mailbox;
            }
        }

        let from = Checker::decode_value(from_addr);
        let subject = Checker::decode_value(envelope.subject);

        let email = format!("{}@{}",
            String::from_utf8_lossy(mailbox.unwrap_or("nobody".as_bytes())),
            String::from_utf8_lossy(host.unwrap_or("nowhere".as_bytes())));

        let subject = subject.unwrap_or("No subject".into());
        let notify = if let Some(from) = from {
            format!("*{}*\n{}\n{}", from, email, subject)
        } else {
            format!("*{}*\n{}", email, subject)
        };

        let bot = bot.clone();
        let username = username.clone();
        tokio::runtime::Runtime::new().unwrap().block_on(async move {
            let res = bot.send_markdown(&username, &notify).await;
            if res.is_err() {
                error!(target: "TelegramBot", "{}", res.unwrap_err());
            }
        });
    }

    fn process_account(
        username: &String,
        account: &MailAccount,
        bot: &TelegramBot
    ) {
        let MailAccount {email, password} = account;
        let client = Checker::new().unwrap().build_client().unwrap();
        let mut session = match client.login(email, password).map_err(|e| e.0) {
            Ok(session) => session,
            Err(e) => {
                error!("Could not login into {}: {}", email, e);
                return;
            }
        };

        let folders = session.list(None, Some("INBOX*")).unwrap();
        for folder in folders.iter() {
            let _mailbox = session.select(folder.name()).unwrap();
            let unseen = session.search("UNSEEN").unwrap();

            if unseen.len() == 0 {
                continue;
            }

            let available_uids = Vec::from_iter(unseen.iter());
            let to_fetch_uids = STORAGE.filter_unprocessed(username, available_uids.as_slice()).unwrap();

            let to_fetch = format!("{}", Vec::from_iter(to_fetch_uids.iter().map(|x| x.to_string())).join(","));
            debug!("User: \"{}\" To fetch {}", username, to_fetch);

            let fetched = session.fetch(to_fetch, "ENVELOPE").unwrap();
            for message in fetched.iter() {
                Checker::process_message(message, bot, username);
            }

            STORAGE.add_processed_mails(username, to_fetch_uids.as_slice()).unwrap();
        }

        session.logout().unwrap()
    }

    fn check_on_cron() {
        let bot = TelegramBot::new(Arc::new(STORAGE.deref().clone()));
        let users = STORAGE.get_usernames_for_checking();
        if let Ok(users) = &users {
            for user in users {
                let account = STORAGE.get_mail_account(user);
                if account.is_none() {
                    error!(target: "MailChecker", "There is no valid mail account for user {}", user);
                    STORAGE.disable_checking(user).unwrap();
                    continue;
                }

                let account = account.unwrap();
                Checker::process_account(user, &account, &bot);
            }

        } else {
            error!(target: "MailChecker", "{}", users.unwrap_err());
        }
    }

    pub fn start() -> Result<()> {
        let mut agenda = schedule::Agenda::new();

        agenda.add(move ||{
            Checker::check_on_cron();
        }).schedule("0 * * * * *")?;

        std::thread::spawn(move || {
            loop {
                agenda.run_pending();
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
       });

        Ok(())
    }
}
