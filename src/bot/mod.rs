use futures::{StreamExt};
use tokio;
use telegram_bot::*;
use regex;
use lazy_static::*;
use std::str::FromStr;
use std::sync::Once;
use log::{error};

use crate::cfg::CONFIG;
use crate::storage::Storage;
use crate::types::{Error, Result};

#[derive(Clone)]
pub struct TelegramBot {
    api: Api,
    storage: Storage,
}

impl TelegramBot {
    pub fn new(storage: Storage) -> TelegramBot {
        let token = CONFIG.get::<String>("bot.secret");
        let api = Api::new(token);

        let bot = TelegramBot{
            api,
            storage
        };

        let bot_ = bot.clone();

        static INIT_LISTENER: Once = Once::new();
        INIT_LISTENER.call_once(|| {
            tokio::spawn(async move { bot_.run().await });
        });

        bot
    }

    async fn run(&self) {
        let mut stream = self.api.stream();

        while let Some(update) = stream.next().await {
            if let Ok(update) = update {
                if let UpdateKind::Message(message) = update.kind {
                    if let MessageKind::Text { ref data, .. } = message.kind {
                        self.process_text(&message, data).await;
                    }
                }
            } else {
                error!("{}", update.unwrap_err());
            }
        }
    }

    async fn process_text(&self, message: &Message, text: &String) {
        lazy_static! {
            static ref ATTACH_REGEX: regex::Regex = regex::Regex::new(r"/attach (\d{6})").unwrap();
        }
        if ATTACH_REGEX.is_match(text.as_str()) {
            let captures = ATTACH_REGEX.captures(text.as_str()).unwrap();
            let code = String::from(captures.get(1).unwrap().as_str());

            let request = self.storage.get_attach_request(&code);
            if let Some(request) = request {
                if request.is_valid() {
                    self.storage.set_telegram_id(&request, &message.chat.id().to_string());
                    let chat = message.chat.clone();
                    self.api.send(chat.text("Success")).await.unwrap();
                }
            }
        }
    }

    pub async fn send_login_code(&self, chat_id: String, username: &String) {
        let chat_id: Integer = Integer::from_str(&chat_id.as_str()).unwrap();
        let chat = self.api.send(GetChat::new(ChatRef::from_chat_id(ChatId::new(chat_id)))).await.unwrap();
        let code = self.storage.create_login_request(username);
        self.api.send(chat.text(format!("Your login code: {}", code))).await.unwrap();
    }

    pub async fn send_markdown(&self, username: &String, text: &String) -> Result<()> {
        let chat_id_str = self.storage.get_telegram_id(username)?;

        let chat_id: Integer = Integer::from_str(&chat_id_str.as_str())?;
        let chat = self.api.send(GetChat::new(ChatRef::from_chat_id(ChatId::new(chat_id)))).await?;

        self.api.send(chat.text(text).parse_mode(ParseMode::Markdown)).await?;

        Ok(())
    }
}
