mod bot;
mod handlers;

use common::cfg::build_config;
use tracing::error;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

use common::ctrlc_handler::set_ctrlc_handler;
use common::heartbeat::HeartbeatService;
use common::storage::Storage;
use common::types::*;

async fn main_impl() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let cfg = build_config()?;

    set_ctrlc_handler(r)?;

    let storage: Pin<Arc<Storage>> = Arc::pin(Storage::new(&cfg).await?);
    let bot = Arc::new(bot::TelegramBot::new(storage.clone(), &cfg, running));

    let heartbeat_service = HeartbeatService::new("TELEGRAM_BOT".into(), storage.clone());
    heartbeat_service.run();

    let cloned_bot = bot.clone();
    tokio::spawn(async move { cloned_bot.start_listener_thread().await });
    bot.start_message_queue_listener_thread().await;

    Ok(())
}

fn main() {
    let _guard = common::sentry::init_sentry();

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_filter(LevelFilter::WARN);

    Registry::default()
        .with(sentry::integrations::tracing::layer())
        .with(fmt_layer)
        .try_init()
        .unwrap();

    tokio::runtime::Runtime::new()
        .expect("Could not initialize asynchronous runtime")
        .block_on(async move {
            if let Err(e) = main_impl().await {
                error!("TelegramBot finished with error: {}", e);
            }
        });
}
