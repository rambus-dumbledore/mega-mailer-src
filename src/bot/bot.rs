use anyhow::Context;
use tracing::{error, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use teloxide::{
    dispatching::UpdateFilterExt,
    prelude::*,
    types::ParseMode::MarkdownV2,
    types::{KeyboardButton, KeyboardMarkup, Message},
};

use common::storage::Storage;
use common::types::Error;

use crate::cfg::TelegramBotCfg;

use super::handlers;
use std::pin::Pin;

#[derive(Clone)]
pub struct TelegramBot {
    bot: Bot,
    storage: Pin<Arc<Storage>>,
    running: Arc<AtomicBool>,
}

impl TelegramBot {
    pub fn new(storage: Pin<Arc<Storage>>, cfg: &TelegramBotCfg, running: Arc<AtomicBool>) -> TelegramBot {
        let token = cfg.bot.token.clone();
        let bot = Bot::new(token);

        TelegramBot {
            bot,
            storage,
            running,
        }
    }

    async fn fetch_all_endpoint(
        bot: Bot,
        msg: Message,
        storage: Pin<Arc<Storage>>,
    ) -> Result<(), Error> {
        handlers::process_fetch_all_emails(bot, msg, storage).await?;
        Ok(())
    }

    pub async fn start_listener_thread(&self) {
        let messages_handler = Update::filter_message()
            .branch(
                dptree::filter(|msg: Message| msg.text().eq(&Some("Fetch all emails")))
                    .endpoint(TelegramBot::fetch_all_endpoint),
            );

        let storage = self.storage.clone();
        let bot_name = String::from("");
        let bot = self.bot.clone();

        Dispatcher::builder(bot, messages_handler)
            .dependencies(dptree::deps![storage, bot_name])
            .default_handler(|upd| async move {
                warn!("Unhandled update: {:?}", upd);
            })
            // If the dispatcher fails for some reason, execute this handler.
            .error_handler(LoggingErrorHandler::with_custom_text(
                "An error has occurred in the dispatcher",
            ))
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    }

    pub async fn start_message_queue_listener_thread(&self) {
        loop {
            let queue = self
                .storage
                .get_send_message_tasks_queue().await
                .with_context(|| "Could not fetch message queue");
            
            if let Err(e) = queue {
                error!("Error occured: {}", e);
                continue;
            }

            for (ref key, ref task) in queue.unwrap() {
                if !task.important && !task.can_send_now() {
                    continue;
                }

                match TelegramBot::send_markdown(&self.bot,  task.to, &task.text)
                    .await
                {
                    Err(e) => {
                        error!("{}", e);
                    }
                    Ok(_) => {
                        match self.storage.remove_send_message_task_from_queue(key).await {
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

    pub async fn send_markdown(
        bot: &Bot,
        user_id: UserId,
        text: &String,
    ) -> Result<(), Error> {
        let reply_markup = KeyboardMarkup::new(vec![vec![KeyboardButton {
            text: "Fetch all emails".into(),
            request: None,
        }]])
            .resize_keyboard(true);
        let chat_id: ChatId = user_id.into();
        bot.send_message(chat_id, text)
            .parse_mode(MarkdownV2)
            .reply_markup(reply_markup)
            .send()
            .await?;
        Ok(())
    }
}
