use std::sync::Arc;
use teloxide::prelude::{*, Message as TelegramMessage};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::collections::BTreeMap;

use common::cfg::CONFIG;
use common::storage::Storage;
use common::types::Error;
use crate::bot::TelegramBot;

pub async fn process_set_avatar_command(
    cx: UpdateWithCx<Bot, TelegramMessage>,
    storage: Arc<Storage>,
) -> Result<(), Error> {
    let username = storage.get_username(&cx.chat_id().to_string());
    if let Err(_) = username {
        cx.answer("Should register first").send().await?;
        return Ok(());
    }
    let username = username.unwrap();

    let avatars = cx
        .requester
        .get_user_profile_photos(cx.chat_id())
        .send()
        .await?;
    if avatars.total_count != 0 {
        let avatar = &avatars.photos[0][0];
        let avatar_file = cx.requester.get_file(&avatar.file_id).send().await?;
        let url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            cx.requester.token(),
            avatar_file.file_path
        );
        let data = reqwest::get(url).await?;

        let file_name = format!("{}.jpg", uuid::Uuid::new_v4());
        let file_storage_path = CONFIG.get::<String>("file_storage.path");
        let mut file = File::create(format!("{}/{}", file_storage_path, file_name)).await?;
        let data = data.bytes().await?;
        file.write(data.as_ref()).await?;

        storage.set_user_avatar(&username, &file_name)?;
    }
    cx.answer("Success").send().await?;
    Ok(())
}

pub async fn process_attach_command(
    cx: UpdateWithCx<Bot, TelegramMessage>,
    storage: Arc<Storage>,
    code: &String,
) -> Result<(), Error> {
    let request = storage.get_attach_request(code);
    if let Some(request) = request {
        if request.is_valid() {
            let chat_id = cx.chat_id().to_string();
            storage.set_telegram_id(&request, &chat_id);
            cx.answer(format!("Success")).send().await?;
            return Ok(());
        }
    }
    cx.answer("Invalid code").send();
    Ok(())
}

pub async fn process_fetch_all_emails(
    cx: UpdateWithCx<Bot, TelegramMessage>,
    storage: Arc<Storage>,
) -> Result<(), Error> {
    let chat_id = cx.chat_id().to_string();
    let username = storage.get_username(&chat_id)?;
    let queue = storage.get_send_message_tasks_queue()?;
    let queue = queue.iter().filter(|(ref _key, ref task)| task.to == username ).collect::<BTreeMap<_,_>>();

    if queue.len() == 0 {
        TelegramBot::send_markdown(
            &cx.requester, &storage, &username, &String::from("There are no messages for you now")).await?;
    } else {
        for (ref key, ref task) in queue {
            if task.to != username { continue; }
            TelegramBot::send_markdown(&cx.requester, &storage, &username, &task.text).await?;
            storage.remove_send_message_task_from_queue(key)?;
        }
    }
    Ok(())
}