mod cfg;
mod checker;

use anyhow::{anyhow, Context};
use clokwerk::TimeUnits;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

use checker::Checker;
use common::cfg::build_config;
use common::ctrlc_handler::set_ctrlc_handler;
use common::heartbeat::HeartbeatService;
use common::storage::Storage;

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

    let moscow_offset =
        chrono::FixedOffset::east_opt(3 * 3600).ok_or(anyhow!("Could not create FixedOffset"))?;

    let mut scheduler = clokwerk::AsyncScheduler::with_tz(moscow_offset);

    async fn task(cfg: Arc<MailCheckerCfg>) {
        let checker = Checker::new(&cfg)
            .await
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
    let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);

    async fn emit_task(tx: tokio::sync::mpsc::Sender<()>) {
        if let Err(e) = tx.send(()).await {
            tracing::error!("tx.send() finished with error: {}", e);
        }
    }

    async fn receive_task(
        mut rx: tokio::sync::mpsc::Receiver<()>,
        running: Arc<AtomicBool>,
        cfg: Arc<MailCheckerCfg>,
    ) {
        while running.load(Ordering::Relaxed) {
            match rx.recv().await {
                Some(_) => {
                    task(cfg.clone()).await;
                }
                None => {}
            }
        }
    }

    let ls = tokio::task::LocalSet::new();
    let handle = ls.spawn_local(receive_task(rx, running.clone(), cfg.clone()));

    let tx = tx.clone();
    scheduler
        .every(1.minute())
        .run(move || emit_task(tx.clone()));

    ls.run_until(async move {
        while running.load(Ordering::Relaxed) {
            scheduler.run_pending().await;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    })
    .await;

    let _ = handle.await;

    Ok(())
}

fn main() {
    let _guard = common::sentry::init_sentry();

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_filter(LevelFilter::INFO);

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
