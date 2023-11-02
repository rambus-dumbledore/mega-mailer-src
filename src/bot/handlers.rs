use std::collections::BTreeMap;
use std::sync::Arc;
use teloxide::prelude::*;

use crate::bot::TelegramBot;
use common::storage::Storage;
use common::types::{Error, InternalError};
use std::pin::Pin;

pub async fn process_fetch_all_emails(
    bot: Bot,
    msg: Message,
    storage: Pin<Arc<Storage>>,
) -> Result<(), Error> {
    let chat_id = msg.chat.id;
    let user_id: UserId = match chat_id.is_user() {
        true => UserId(chat_id.0 as u64),
        _ => return Err(Error::InternalError(InternalError::RuntimeError(format!("ChatId is not belongs to user: {}", chat_id))))
    };
    let queue = storage.get_send_message_tasks_queue().await?;
    let queue = queue
        .iter()
        .filter(|(ref _key, ref task)| task.to == user_id)
        .collect::<BTreeMap<_, _>>();

    if queue.len() == 0 {
        TelegramBot::send_markdown(
            &bot,
            user_id,
            &String::from("There are no messages for you now"),
        )
        .await?;
    } else {
        for (ref key, ref task) in queue {
            if task.to != user_id {
                continue;
            }
            TelegramBot::send_markdown(&bot,  user_id, &task.text).await?;
            storage.remove_send_message_task_from_queue(key).await?;
        }
    }
    Ok(())
}
