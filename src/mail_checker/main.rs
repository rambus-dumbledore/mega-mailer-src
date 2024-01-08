mod checker;
mod cfg;

use anyhow::{anyhow, Context};
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
use common::cfg::build_config;

use crate::cfg::MailCheckerCfg;

async fn main_impl() -> anyhow::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_ctrlc_handler(r)?;
    let cfg = Arc::new(build_config::<MailCheckerCfg>()?);

    let storage: Pin<Arc<Storage>> = Arc::pin(Storage::new(&cfg.storage).await?);

    let heartbeat_service = HeartbeatService::new("MAIL_CHECKER".into(), storage);
    heartbeat_service.run();

    tracing::info!("Started mail checker");

    let moscow_offset = chrono::FixedOffset::east_opt(3 * 3600).ok_or(anyhow!("Could not create FixedOffset"))?;

    let mut scheduler = clokwerk::AsyncScheduler::with_tz(moscow_offset);
    
    async fn task(cfg: Arc<MailCheckerCfg>) {
        let checker = Checker::new(&cfg).await
            .with_context(|| "Cound not create checker");
        let checker = match checker {
            Ok(checker) => checker,
            Err(e) => {
                tracing::error!("{}", e);
                return;
            }
        };
        checker.check_on_cron().await;
    }

    scheduler.every(1.minute()).run(move || task(cfg.clone()));

    while running.load(Ordering::Relaxed) {
        scheduler.run_pending().await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(main_impl()) {
        tracing::error!("MailChecker finished with error: {}", e)
    };
}
