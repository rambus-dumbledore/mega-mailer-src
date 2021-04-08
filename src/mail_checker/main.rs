mod checker;

use ctrlc;
use log::error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use checker::Checker;
use common::types::InternalError::RuntimeError;
use common::types::*;

fn main_impl() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .map_err(|e| {
        Error::InternalError(RuntimeError(format!("Error setting signal handler: {}", e)))
    })?;

    let mut agenda = schedule::Agenda::new();

    agenda
        .add(move || {
            Checker::check_on_cron();
        })
        .schedule("0 * * * * *")?;

    while running.load(Ordering::Relaxed) {
        agenda.run_pending();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}

fn main() {
    match main_impl() {
        Ok(_) => {}
        Err(e) => {
            error!("MailChecker finished with error: {}", e)
        }
    }
}
