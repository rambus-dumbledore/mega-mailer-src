mod checker;

use clokwerk::TimeUnits;
use log::error;
use pretty_env_logger;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use checker::Checker;
use common::ctrlc_handler::set_ctrlc_handler;
use common::heartbeat::HeartbeatService;
use common::storage::Storage;
use common::types::*;

fn main_impl() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_ctrlc_handler(r)?;

    let storage: Pin<Arc<Storage>> = Arc::pin(Storage::new()?);

    let heartbeat_service = HeartbeatService::new("MAIL_CHECKER".into(), storage);
    heartbeat_service.run();

    let checker = Arc::pin(Checker::new());

    let mut scheduler = clokwerk::Scheduler::with_tz(chrono::FixedOffset::east(3 * 3600));
    scheduler.every(1.minute()).run(move || {
        checker.check_on_cron();
    });

    while running.load(Ordering::Relaxed) {
        scheduler.run_pending();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}

fn main() {
    pretty_env_logger::init();

    let _guard = common::sentry::init_sentry();

    if let Err(e) = main_impl() {
        error!("MailChecker finished with error: {}", e)
    };
}
