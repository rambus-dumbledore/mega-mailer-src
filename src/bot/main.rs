mod bot;
mod handlers;

use common::storage::Storage;
use common::types::*;
use log::error;

async fn main_impl() -> Result<()> {
    let storage = Storage::new()?.into();
    let bot = bot::TelegramBot::new(storage);

    bot.start_listener_thread();
    bot.start_message_queue_listener_thread().await;

    Ok(())
}

fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        match main_impl().await {
            Ok(_) => {}
            Err(e) => {
                error!("TelegramBot finished with error: {}", e)
            }
        }
    });
}
