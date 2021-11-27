mod bot;
mod handlers;

use log::error;
use std::sync::atomic::{AtomicBool};
use std::sync::Arc;
use std::pin::Pin;

use common::storage::Storage;
use common::types::*;
use common::heartbeat::HeartbeatService;
use common::ctrlc_handler::set_ctrlc_handler;

async fn main_impl() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_ctrlc_handler(r)?;

    let storage: Pin<Arc<Storage>> = Arc::pin(Storage::new()?);
    let bot = bot::TelegramBot::new(storage.clone(), running);

    let heartbeat_service = HeartbeatService::new("TELEGRAM_BOT".into(), storage.clone());
    heartbeat_service.run();

    bot.start_listener_thread();
    bot.start_message_queue_listener_thread().await;

    Ok(())
}

fn main() {
    let _guard = common::sentry::init_sentry();

    tokio::runtime::Runtime::new()
        .expect("Could not initialize asynchronous runtime")
        .block_on(async move {
        teloxide::enable_logging!();
        if let Err(e) = main_impl().await {
            error!("TelegramBot finished with error: {}", e);
        }
    });
}
