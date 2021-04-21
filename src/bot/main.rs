mod bot;
mod handlers;

use ctrlc;
use log::error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use common::storage::Storage;
use common::types::*;

async fn main_impl() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .map_err(|e| {
        Error::InternalError(InternalError::RuntimeError(format!("Error setting signal handler: {}", e)))
    })?;

    let storage = Storage::new()?.into();
    let bot = bot::TelegramBot::new(storage, running);

    bot.start_listener_thread();
    bot.start_message_queue_listener_thread().await;

    Ok(())
}

fn main() {
    tokio::runtime::Runtime::new()
        .expect("Could not initialize asynchronous runtime")
        .block_on(async move {
        teloxide::enable_logging!();
        match main_impl().await {
            Ok(_) => {}
            Err(e) => {
                error!("TelegramBot finished with error: {}", e)
            }
        }
    });
}
