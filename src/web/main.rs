mod account_handlers;
mod auth_handlers;
mod heartbeat_handlers;
mod importance_settings_handlers;
mod notify_settings_handlers;
mod server;
mod cfg;

use std::sync::Arc;
use axum::Extension;
use anyhow::Result;
use cfg::WebServerCfg;
use tower_cookies::CookieManagerLayer;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

use common::cfg::build_config;
use common::storage::{Storage, Cipher};

use server::init_server_instance;

#[derive(rust_embed::RustEmbed)]
#[folder = "src/db/"]
struct SQLMigration;

async fn main_impl() -> Result<()> {
    let cfg = Arc::new(build_config::<WebServerCfg>()?);
    let storage = Arc::new(Storage::new(&cfg.storage).await?);
    let cipher = Arc::new(Cipher::new(&cfg.storage));

    let sql = SQLMigration::get("pg_init.sql").expect("There is no pg migration file");
    storage.migrate_pg(&std::str::from_utf8(sql.data.as_ref())?).await?;

    let (router, address) = init_server_instance(&cfg.web).await;
    let app = router
        .layer(Extension(storage))
        .layer(Extension(cipher))
        .layer(Extension(cfg))
        .layer(CookieManagerLayer::new());

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app.into_make_service())
        .await?;
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

    let rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = rt.block_on(main_impl()) {
        tracing::error!("Web finished with error: {}", e);
    }
}
