use common::queues::{Queue, TelegramMessageTask};
use tokio::select;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use teloxide::{
    dispatching::UpdateFilterExt,
    prelude::*,
    types::ParseMode::MarkdownV2,
    types::{KeyboardButton, KeyboardMarkup, Message},
};
use tokio::sync::RwLock;
use tracing::{error, warn};

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
    queue: Queue,
    tasks: Arc<RwLock<HashMap<u64, (TelegramMessageTask, u16)>>>
}

impl TelegramBot {
    pub fn new(
        storage: Pin<Arc<Storage>>,
        cfg: &TelegramBotCfg,
        queue: Queue,
        running: Arc<AtomicBool>,
        tasks: Arc<RwLock<HashMap<u64, (TelegramMessageTask, u16)>>>
    ) -> TelegramBot {
        let token = cfg.bot.token.clone();
        let bot = Bot::new(token);

        TelegramBot {
            bot,
            storage,
            running,
            queue,
            tasks,
        }
    }

    async fn fetch_all_endpoint(
        bot: Bot,
        msg: Message,
        queue: Queue,
        tasks: Arc<RwLock<HashMap<u64, (TelegramMessageTask, u16)>>>
    ) -> Result<(), Error> {
        handlers::process_fetch_all_emails(bot, msg, queue, tasks).await?;
        Ok(())
    }

    pub async fn start_listener_thread(&self) {
        let messages_handler = Update::filter_message().branch(
            dptree::filter(|msg: Message| msg.text().eq(&Some("Fetch all emails")))
                .endpoint(TelegramBot::fetch_all_endpoint),
        );

        let storage = self.storage.clone();
        let bot_name = String::from("");
        let bot = self.bot.clone();
        let queue = self.queue.clone();
        let tasks = self.tasks.clone();

        Dispatcher::builder(bot, messages_handler)
            .dependencies(dptree::deps![storage, queue, tasks, bot_name])
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
        let tm = self.tasks.clone();
        let queue = self.queue.clone();
        tokio::spawn(async move {
            loop {
                let (mut rx, mut thread) = if let Ok((rx, thread)) = queue.subscribe("telegram_bot".into()).await {
                    (rx, thread)
                } else {
                    return;
                };

                loop {
                    select! {
                        Some((msg, delivery_tag, channel_id)) = rx.recv() => {
                            match msg {
                                common::queues::QueueMessage::Tasks(tasks) => match tasks {
                                    common::queues::Tasks::TelegramMessageTask(task) => {
                                        tm.write().await.insert(delivery_tag, (task, channel_id));
                                    }
                                },
                            }
                        },
                        result = &mut thread => {
                            if let Err(e) = result {
                                tracing::error!("Consumer thread finished with error: {}, restarting thread...", e);
                            }
                            break;
                        }
                    }
                }
            }
        });

        loop {
            let mut to_remove = Vec::new();
            
            {
                let lock = self.tasks.read().await;
                let tasks: Vec<(u64, TelegramMessageTask, u16)> = lock.iter()
                    .map(|(delivery_tag, (task, channel_id))| (*delivery_tag, task.clone(), *channel_id))
                    .collect();
                for (delivery_tag, task, channel_id) in tasks {

                    if !task.important && !task.can_send_now() {
                        continue;
                    }
    
                    match TelegramBot::send_markdown(&self.bot, task.to, &task.text).await {
                        Err(e) => {
                            error!("{}", e);
                        }
                        Ok(_) => {
                            match self.queue.ack(delivery_tag, channel_id).await {
                                Err(e) => {
                                    error!("{}", e);
                                }
                                _ => {}
                            };
                            to_remove.push(delivery_tag);
                        }
                    };
                }
            }

            if !to_remove.is_empty() {
                let mut map = self.tasks.write().await;
                for delivery_tag in to_remove {
                    map.remove(&delivery_tag);
                }
            }

            if !self.running.load(Ordering::Relaxed) {
                break;
            }

            let _ = tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    }

    pub async fn send_markdown(bot: &Bot, user_id: UserId, text: &String) -> Result<(), Error> {
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
