use imap;
use lazy_static::lazy_static;
use log::{debug, error};
use rustls_connector::{RustlsConnector, TlsStream};
use rustyknife::rfc2047::encoded_word;
use std::iter::FromIterator;
use std::net::TcpStream;
use teloxide::utils::markdown::escape;
use chrono::{Timelike, DateTime, Utc, TimeZone, Duration};

use common::cfg::CONFIG;
use common::storage::{MailAccount, Storage};
use common::types::{Error, MailCheckerError, Result, TelegramMessageTask, ImportanceChecker};

lazy_static! {
    static ref STORAGE: Storage = Storage::new().unwrap();
}

pub struct Checker {
    host: String,
    port: u16,
}

impl Checker {
    fn new() -> Checker {
        let host = CONFIG.get::<String>("mail.address");
        let port = CONFIG.get::<u16>("mail.port");
        Checker { host, port }
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

    fn process_message(message: &imap::types::Fetch, username: &String, importance_checker: &ImportanceChecker) {
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

        let email = format!(
            "{}@{}",
            String::from_utf8_lossy(mailbox.unwrap_or("nobody".as_bytes())),
            String::from_utf8_lossy(host.unwrap_or("nowhere".as_bytes()))
        );

        let subject = subject.unwrap_or("No subject".into());
        let text = if let Some(from) = from {
            format!(
                "*{}*\n{}\n{}",
                escape(from.as_str()),
                escape(email.as_str()),
                escape(subject.as_str())
            )
        } else {
            format!("*{}*\n{}", escape(email.as_str()), escape(subject.as_str()))
        };

        let work_hours =  STORAGE.get_user_working_hours(&username);
        let send_after = if let Some(work_hours) = work_hours {
            let moscow_offset = chrono::FixedOffset::east(3 * 3600);
            let now = chrono::Utc::now().with_timezone(&moscow_offset);
            let mut send_after: DateTime<Utc> = DateTime::from(now);
            if (now.hour() as u8) < work_hours[0]  {
                let naive = now.naive_utc().date();
                send_after = Utc.from_utc_date(&naive)
                    .and_hms(work_hours[0] as u32, 0, 0) - Duration::hours(3);
            } else if (now.hour() as u8) >= work_hours[1] {
                let naive = now.naive_utc().date();
                send_after = (Utc.from_utc_date(&naive) + Duration::days(1))
                    .and_hms(work_hours[0] as u32, 0, 0) - Duration::hours(3);
            }
            send_after
        } else {
            chrono::Utc::now()
        };

        let task = TelegramMessageTask {
            to: username.clone(),
            text,
            send_after,
            important: importance_checker.check(&email, &subject),
        };
        if let Err(e) = STORAGE.add_send_message_task_to_queue(task) {
            error!("{}", e);
            return;
        }
    }

    fn process_account(username: &String, account: &MailAccount) {
        let MailAccount { email, password } = account;
        let client = match Checker::new().build_client() {
            Ok(client) => client,
            Err(e) => {
                error!("Could not connect to mail server: {}", e);
                return;
            }
        };

        let mut session = match client.login(email, password).map_err(|e| e.0) {
            Ok(session) => session,
            Err(e) => {
                error!("Could not login into {}: {}", email, e);
                return;
            }
        };

        let importance_checker = ImportanceChecker::new(&*STORAGE, username);
        debug!("ImportanceChecker for user {} was built: {:?}", username, importance_checker);

        let folders = session.list(None, Some("INBOX*")).unwrap();
        for folder in folders.iter() {
            let _mailbox = session.select(folder.name()).unwrap();
            let unseen = session.search("UNSEEN").unwrap();

            if unseen.len() == 0 {
                continue;
            }

            let available_uids = Vec::from_iter(unseen.iter());
            let to_fetch_uids = STORAGE
                .filter_unprocessed(username, available_uids.as_slice())
                .unwrap();

            let to_fetch = format!(
                "{}",
                Vec::from_iter(to_fetch_uids.iter().map(|x| x.to_string())).join(",")
            );
            debug!("User: \"{}\" To fetch {}", username, to_fetch);

            let fetched = session.fetch(to_fetch, "ENVELOPE").unwrap();
            for message in fetched.iter() {
                Checker::process_message(message, username, &importance_checker);
            }

            STORAGE
                .add_processed_mails(username, to_fetch_uids.as_slice())
                .unwrap();
        }

        session.logout().unwrap()
    }

    pub fn check_on_cron() {
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
                Checker::process_account(user, &account);
            }
        } else {
            error!(target: "MailChecker", "{}", users.unwrap_err());
        }
    }
}
