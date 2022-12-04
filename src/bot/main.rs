mod bot;
mod handlers;

use log::error;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use common::ctrlc_handler::set_ctrlc_handler;
use common::heartbeat::HeartbeatService;
use common::storage::Storage;
use common::types::*;

async fn main_impl() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_ctrlc_handler(r)?;

    let storage: Pin<Arc<Storage>> = Arc::pin(Storage::new()?);
    let bot = Arc::new(bot::TelegramBot::new(storage.clone(), running));

    let heartbeat_service = HeartbeatService::new("TELEGRAM_BOT".into(), storage.clone());
    heartbeat_service.run();

    let cloned_bot = bot.clone();
    tokio::spawn(async move { cloned_bot.start_listener_thread().await });
    bot.start_message_queue_listener_thread().await;

    Ok(())
}

fn main() {
    pretty_env_logger::init();
    let _guard = common::sentry::init_sentry();

    tokio::runtime::Runtime::new()
        .expect("Could not initialize asynchronous runtime")
        .block_on(async move {
            if let Err(e) = main_impl().await {
                error!("TelegramBot finished with error: {}", e);
            }
        });
}
