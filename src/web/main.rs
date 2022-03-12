mod account_handlers;
mod auth_handlers;
mod heartbeat_handlers;
mod importance_settings_handlers;
mod notify_settings_handlers;
mod server;

use axum::AddExtensionLayer;
use log::error;
use pretty_env_logger;
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;

use common::sessions::SessionKeystore;
use common::storage::Storage;

use server::init_server_instance;

fn main() {
    pretty_env_logger::init();

    let _guard = common::sentry::init_sentry();

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
            .layer(AddExtensionLayer::new(storage))
            .layer(AddExtensionLayer::new(session_keystore))
            .layer(CookieManagerLayer::new());

        let addr: std::net::SocketAddr = format!("{}:{}", address, port).parse().unwrap();
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    })
}
