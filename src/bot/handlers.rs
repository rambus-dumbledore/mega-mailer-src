use common::queues::{BrokerClient, TelegramMessageTask};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::RwLock;

use crate::bot::TelegramBot;
use common::types::{Error, InternalError};

pub async fn process_fetch_all_emails(
    bot: Bot,
    msg: Message,
    _broker_client: BrokerClient,
    tasks: Arc<RwLock<HashMap<uuid::Uuid, TelegramMessageTask>>>,
) -> Result<(), Error> {
    let chat_id = msg.chat.id;
    let user_id: UserId = match chat_id.is_user() {
        true => UserId(chat_id.0 as u64),
        _ => {
            return Err(Error::InternalError(InternalError::RuntimeError(format!(
                "ChatId is not belongs to user: {}",
                chat_id
            ))))
        }
    };
    let tasks = tasks.read().await;
    let tasks = tasks
        .iter()
        .filter(|(ref _key, ref task)| task.to == user_id)
        .collect::<BTreeMap<_, _>>();

    if tasks.len() == 0 {
        TelegramBot::send_markdown(
            &bot,
            user_id,
            &String::from("There are no messages for you now"),
        )
        .await?;
    } else {
        for (_task_id, task) in tasks {
            if task.to != user_id {
                continue;
            }
            TelegramBot::send_markdown(&bot, user_id, &task.text).await?;
            // if let Err(e) = queue.ack(*delivery_tag, *channel_id).await {
            //    tracing::warn!("queue.ack() finished with error: {e}");
            //}
        }
    }
    Ok(())
}
