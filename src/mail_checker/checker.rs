use chrono::Timelike;
use imap;
use rustls_connector::{RustlsConnector, TlsStream};
use rustyknife::rfc2047::encoded_word;
use std::iter::FromIterator;
use std::net::TcpStream;
use teloxide::utils::markdown::escape;
use anyhow::{Context, anyhow};
use std::sync::Arc;

use common::cfg::CONFIG;
use common::storage::{MailAccount, Storage};
use common::types::{Error, ImportanceChecker, MailCheckerError, Result, TelegramMessageTask};

pub struct Checker {
    host: String,
    port: u16,
    storage: Arc<Storage>,
}

impl Checker {
    pub fn new() -> anyhow::Result<Checker> {
        let host = CONFIG.get::<String>("mail.address");
        let port = CONFIG.get::<u16>("mail.port");
        let storage = Storage::new().with_context(|| "Could not connect to storage")?.into();
        Ok(Checker {
            host,
            port,
            storage,
        })
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
        &self,
        message: &imap::types::Fetch,
        username: &String,
        importance_checker: &ImportanceChecker,
    ) -> anyhow::Result<()> {
        let envelope = message.envelope();
        if let None = envelope {
            let error = Error::MailCheckerError(MailCheckerError::EmptyEnvelope);
            return Err(anyhow!(error));
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

        let work_hours = self.storage.get_user_working_hours(&username);
        let send_after = if let Some(work_hours) = work_hours {
            let moscow_offset = chrono::FixedOffset::east(3 * 3600);
            let now = chrono::Utc::now().with_timezone(&moscow_offset);

            let from = now
                .with_hour(work_hours[0] as u32).unwrap_or(now)
                .with_minute(0).unwrap()
                .with_second(0).unwrap();
            let to = now
                .with_hour(work_hours[1] as u32).unwrap_or(now)
                .with_minute(0).unwrap()
                .with_second(0).unwrap();
            
            let mut send_after = chrono::Utc::now();
            let utc_offset = chrono::Utc{};
            if from <= now && now <= to {
                send_after = now.with_timezone(&utc_offset)
            } else if to < now {
                send_after = from.checked_add_days(chrono::Days::new(1)).unwrap().with_timezone(&utc_offset)
            } else if now < from {
                send_after = to.with_timezone(&utc_offset)
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

        if let Err(e) = self.storage.add_send_message_task_to_queue(task) {
            return Err(anyhow!(e));
        }

        Ok(())
    }

    fn process_account(&self, username: &String, account: &MailAccount) -> anyhow::Result<()> {
        let MailAccount { email, password } = account;
        let client = match self.build_client() {
            Ok(client) => client,
            Err(e) => {
                return Err(anyhow!("Could not connect to mail server: {}", e));
            }
        };

        let mut session = match client.login(email, password).map_err(|e| e.0) {
            Ok(session) => session,
            Err(e) => {
                return Err(anyhow!("Could not login into {}: {}", email, e));
            }
        };

        let importance_checker = ImportanceChecker::new(&*self.storage, username);
        tracing::debug!(
            "ImportanceChecker for user {} was built: {:?}",
            username, importance_checker
        );

        let folders = session.list(None, Some("INBOX*"))?;
        for folder in folders.iter() {
            let _mailbox = session.select(folder.name())?;
            let unseen = session.search("UNSEEN")?;

            if unseen.len() == 0 {
                continue;
            }

            let available_uids = Vec::from_iter(unseen.iter());
            let to_fetch_uids = self
                .storage
                .filter_unprocessed(username, available_uids.as_slice())?;

            let to_fetch = format!(
                "{}",
                Vec::from_iter(to_fetch_uids.iter().map(|x| x.to_string())).join(",")
            );
            tracing::debug!("User: \"{}\" To fetch {}", username, to_fetch);

            let fetched = session.fetch(to_fetch, "ENVELOPE")?;
            for message in fetched.iter() {
                self.process_message(message, username, &importance_checker)?;
            }

            self.storage
                .add_processed_mails(username, to_fetch_uids.as_slice())?
        }

        session.logout()?;

        Ok(())
    }

    pub fn check_on_cron(&self) {
        let users = self.storage.get_usernames_for_checking();

        if let Ok(users) = &users {
            for user in users {
                let account = self.storage.get_mail_account(user);
                if account.is_none() {
                    tracing::error!("There is no valid mail account for user {}", user);
                    if let Err(e) = self.storage.disable_checking(user).with_context(|| "Failed to disable checking") {
                        tracing::error!("{}", e);
                    }
                    continue;
                }

                let account = account.unwrap();
                if let Err(e) = self.process_account(user, &account) {
                    tracing::error!("{}", e);
                }
            }
        } else {
            tracing::error!("{}", users.unwrap_err());
        }
    }
}
