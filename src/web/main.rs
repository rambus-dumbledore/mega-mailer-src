mod account_handlers;
mod auth_handlers;
mod heartbeat_handlers;
mod importance_settings_handlers;
mod notify_settings_handlers;
mod server;

use axum::Extension;
use tracing::error;
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

use common::sessions::SessionKeystore;
use common::storage::Storage;

use server::init_server_instance;

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
    rt.block_on(async move {
        let session_keystore = SessionKeystore::new();

        let storage = Storage::new();
        if let Err(err) = storage {
            error!("Could not create connection to storage: {}", err);
            return;
        }
        let storage = Arc::new(storage.unwrap());

        let (router, address, port) = init_server_instance().await;
        let app = router
            .layer(Extension(storage))
            .layer(Extension(session_keystore))
            .layer(CookieManagerLayer::new());

        let addr: std::net::SocketAddr = format!("{}:{}", address, port).parse().unwrap();
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    })
}
