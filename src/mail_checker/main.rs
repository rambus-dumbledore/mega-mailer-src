#![feature(trait_alias)]

mod checker;

use ctrlc;
use log::error;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use pretty_env_logger;

use checker::Checker;
use common::types::*;
use common::storage::{Storage};
use common::heartbeat::HeartbeatService;


fn main_impl() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .map_err(|e| {
        Error::InternalError(InternalError::RuntimeError(format!("Error setting signal handler: {}", e)))
    })?;

    let storage: Pin<Arc<Storage>> = Arc::pin(Storage::new()?);

    let heartbeat_service = HeartbeatService::new("MAIL_CHECKER".into(), storage);
    heartbeat_service.run();

    let mut agenda = schedule::Agenda::new();

    let checker = Arc::pin(Checker::new());
    agenda
        .add(move || {
            checker.check_on_cron();
        })
        .schedule("0 * * * * *")?;

    while running.load(Ordering::Relaxed) {
        agenda.run_pending();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}

fn main() {
    pretty_env_logger::init();

    match main_impl() {
        Ok(_) => {}
        Err(e) => {
            error!("MailChecker finished with error: {}", e)
        }
    }
}
