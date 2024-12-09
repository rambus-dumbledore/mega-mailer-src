use common::queues::{BrokerClient, TelegramMessageTask};
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

use common::retry;
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
    broker: BrokerClient,
    tasks: Arc<RwLock<HashMap<uuid::Uuid, TelegramMessageTask>>>,
}

impl TelegramBot {
    pub fn new(
        storage: Pin<Arc<Storage>>,
        cfg: &TelegramBotCfg,
        broker: BrokerClient,
        running: Arc<AtomicBool>,
        tasks: Arc<RwLock<HashMap<uuid::Uuid, TelegramMessageTask>>>,
    ) -> TelegramBot {
        let token = cfg.bot.token.clone();
        let bot = Bot::new(token);

        TelegramBot {
            bot,
            storage,
            running,
            broker,
            tasks,
        }
    }

    async fn fetch_all_endpoint(
        bot: Bot,
        msg: Message,
        broker_client: BrokerClient,
        tasks: Arc<RwLock<HashMap<uuid::Uuid, TelegramMessageTask>>>,
    ) -> Result<(), Error> {
        handlers::process_fetch_all_emails(bot, msg, broker_client, tasks).await?;
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
        let broker = self.broker.clone();
        let tasks = self.tasks.clone();

        Dispatcher::builder(bot, messages_handler)
            .dependencies(dptree::deps![storage, broker, tasks, bot_name])
            .default_handler(|upd| async move {
                tracing::warn!("Unhandled update: {:?}", upd);
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
        let broker = self.broker.clone();
        tokio::spawn(async move {
            loop {
                let mut rx = match broker.subscribe().await {
                    Ok(rx) => rx,
                    Err(_) => return,
                };
                tracing::info!("subscribed");

                loop {
                    let msg = rx.recv().await;
                    tracing::info!("recieved {:?}", msg);
                    match msg {
                        Some(msg) => match msg.payload {
                            common::queues::BrokerMessagePayload::Tasks(t) => match t {
                                common::queues::Tasks::TelegramMessageTask(task) => {
                                    tm.write().await.insert(msg.message_id, task);
                                }
                            },
                        },

                        _ => {
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
                let tasks: Vec<(uuid::Uuid, TelegramMessageTask)> = lock
                    .iter()
                    .map(|(msg_id, task)| (*msg_id, task.clone()))
                    .collect();
                for (msg_id, task) in tasks {
                    if !task.important && !task.can_send_now() {
                        continue;
                    }

                    match TelegramBot::send_markdown(&self.bot, task.to, &task.text).await {
                        Err(e) => {
                            tracing::error!("{}", e);
                        }
                        Ok(_) => {
                            match retry! { self.broker.ack(msg_id).await } {
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to ack message with id {}: {}",
                                        msg_id,
                                        e
                                    );
                                }
                                _ => {}
                            };
                            to_remove.push(msg_id);
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
        .resize_keyboard();
        let chat_id: ChatId = user_id.into();
        bot.send_message(chat_id, text)
            .parse_mode(MarkdownV2)
            .reply_markup(reply_markup)
            .send()
            .await?;
        Ok(())
    }
}
