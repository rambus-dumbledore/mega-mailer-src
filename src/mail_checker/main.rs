mod checker;

use anyhow::Context;
use clokwerk::TimeUnits;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

use checker::Checker;
use common::ctrlc_handler::set_ctrlc_handler;
use common::heartbeat::HeartbeatService;
use common::storage::Storage;

fn main_impl() -> anyhow::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_ctrlc_handler(r)?;

    let storage: Pin<Arc<Storage>> = Arc::pin(Storage::new()?);

    let heartbeat_service = HeartbeatService::new("MAIL_CHECKER".into(), storage);
    heartbeat_service.run();

    let checker = Arc::pin(Checker::new()
        .with_context(|| "Cound not create checker")?);

    tracing::info!("Started mail checker");

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
    let _guard = common::sentry::init_sentry();

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_filter(LevelFilter::WARN);

    Registry::default()
        .with(sentry::integrations::tracing::layer())
        .with(fmt_layer)
        .try_init()
        .unwrap();

    if let Err(e) = main_impl() {
        tracing::error!("MailChecker finished with error: {}", e)
    };
}
