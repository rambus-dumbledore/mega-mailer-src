use log::{error, debug};
use std::sync::Arc;
use teloxide::types::ParseMode::MarkdownV2;
use teloxide::{prelude::*};
use tokio;
use std::sync::atomic::{AtomicBool, Ordering};
use teloxide::types::{KeyboardMarkup, KeyboardButton, Message as TelegramMessage, MessageKind, MediaKind};

use common::cfg::CONFIG;
use common::storage::Storage;
use common::types::{Error,};

use super::handlers;

#[derive(Clone)]
pub struct TelegramBot {
    bot: Bot,
    storage: Arc<Storage>,
    running: Arc<AtomicBool>,
}

enum RawMessage {
    FetchAllMailRequest,
    Other(String),
}

enum Message {
    AttachRequest(String),
    SetAvatarRequest,
    RawMessage(RawMessage)
}

fn parse_text(text: &String) -> Option<Message> {
    let res = teloxide::utils::command::parse_command(text.as_str(), "");
    return if let Some((cmd, args)) = res {
        match cmd {
            "attach" => Some(Message::AttachRequest(args[0].into())),
            "set_avatar" => Some(Message::SetAvatarRequest),
            _ => None
        }
    } else {
        match text.as_str() {
            "Fetch all emails" => {
                Some(Message::RawMessage(RawMessage::FetchAllMailRequest))
            },
            _ => Some(Message::RawMessage(RawMessage::Other(text.clone())))
        }
    }
}

fn parse_update(update: &TelegramMessage) -> Option<Message> {
    return match update.kind {
        MessageKind::Common(ref msg) => {
            match msg.media_kind {
                MediaKind::Text(ref text) => {
                    parse_text(&text.text)
                },
                _ => None
            }
        },
        _ => None
    }
}

impl TelegramBot {
    pub fn new(storage: Arc<Storage>, running: Arc<AtomicBool>) -> TelegramBot {
        let token = CONFIG.get::<String>("bot.secret");
        let bot = Bot::new(token);

        TelegramBot { bot, storage, running }
    }

    pub fn start_listener_thread(&self) {
        let b = self.bot.clone();
        let s = self.storage.clone();

        let closure = move |msg: UpdateWithCx<Bot, TelegramMessage>| {
            let storage = s.clone();
            async move {
                if let Some(message) = parse_update(&msg.update) {
                    match TelegramBot::answer(msg, message, &storage).await {
                        Err(e) => error!("Could not answer request: {}", e),
                        _ => {}
                    }
                }
                respond(())
            }
        };

        tokio::spawn(async move {
            teloxide::repl(b, closure).await;
        });
    }

    pub async fn start_message_queue_listener_thread(&self) {
        loop {
            let queue = self
                .storage
                .get_send_message_tasks_queue()
                .expect("Could not fetch message queue");
            for (ref key, ref task) in queue {
                if !task.can_send_now() {
                    continue;
                }

                match TelegramBot::send_markdown(&self.bot, &self.storage, &task.to, &task.text).await {
                    Err(e) => {
                        error!("{}", e);
                    }
                    Ok(_) => {
                        match self.storage.remove_send_message_task_from_queue(key) {
                            Err(e) => {
                                error!("{}", e);
                            }
                            _ => {}
                        };
                    }
                };
            }

            if !self.running.load(Ordering::Relaxed) {
                break;
            }

            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }

    async fn answer(
        cx: UpdateWithCx<Bot, TelegramMessage>,
        message: Message,
        storage: &Arc<Storage>,
    ) -> Result<(), Error> {
        let storage = storage.clone();
        match message {
            Message::AttachRequest(code) => handlers::process_attach_command(cx, storage, &code).await?,
            Message::SetAvatarRequest => handlers::process_set_avatar_command(cx, storage).await?,
            Message::RawMessage(msg) => {
                match msg {
                    RawMessage::FetchAllMailRequest => handlers::process_fetch_all_emails(cx, storage).await?,
                    RawMessage::Other(raw_message) => debug!("Could not process message '{}'", raw_message)
                }
            }
        };

        Ok(())
    }

    pub async fn send_markdown(bot: &Bot, storage: &Arc<Storage>, username: &String, text: &String) -> Result<(), Error> {
        let chat_id = storage.get_telegram_id(username)?;
        let reply_markup = KeyboardMarkup::new(vec![vec![
            KeyboardButton{
                text: "Fetch all emails".into(),
                request: None
            }
        ]]).resize_keyboard(true);
        bot
            .send_message(chat_id, text)
            .parse_mode(MarkdownV2)
            .reply_markup(reply_markup)
            .send()
            .await?;
        Ok(())
    }
}
