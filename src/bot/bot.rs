use log::error;
use std::sync::Arc;
use teloxide::types::ParseMode::MarkdownV2;
use teloxide::{prelude::*, utils::command::BotCommand};
use tokio;

use common::cfg::CONFIG;
use common::storage::Storage;
use common::types::Error;

use super::handlers;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct TelegramBot {
    bot: Bot,
    storage: Arc<Storage>,
    running: Arc<AtomicBool>,
}

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "attach telegram account to user account")]
    Attach(String),
    #[command(description = "set account avatar from telegram")]
    SetAvatar,
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

        tokio::spawn(async move {
            teloxide::commands_repl(b, "MegaMailerBot", move |cx, command| {
                TelegramBot::answer(cx, command, Arc::clone(&s))
            })
            .await;
        });
    }

    pub async fn start_message_queue_listener_thread(&self) {
        loop {
            let queue = self
                .storage
                .get_send_message_tasks_queue()
                .expect("Could not fetch message queue");
            for (ref key, ref task) in queue {
                match self.send_markdown(&task.to, &task.text).await {
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

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    async fn answer(
        cx: UpdateWithCx<Bot, Message>,
        command: Command,
        storage: Arc<Storage>,
    ) -> Result<(), Error> {
        match command {
            Command::Help => {
                cx.answer(Command::descriptions()).send();
            }
            Command::Attach(code) => handlers::process_attach_command(cx, &storage, &code).await?,
            Command::SetAvatar => handlers::process_set_avatar_command(cx, &storage).await?,
        };

        Ok(())
    }

    pub async fn send_login_code(&self, chat_id: String, username: &String) -> Result<(), Error> {
        let code = self.storage.create_login_request(username);
        self.bot
            .send_message(chat_id, format!("Your login code: {}", code))
            .send()
            .await?;
        Ok(())
    }

    pub async fn send_markdown(&self, username: &String, text: &String) -> Result<(), Error> {
        let chat_id = self.storage.get_telegram_id(username)?;
        self.bot
            .send_message(chat_id, text)
            .parse_mode(MarkdownV2)
            .send()
            .await?;
        Ok(())
    }
}
