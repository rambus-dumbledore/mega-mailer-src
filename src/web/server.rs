use axum::{
    routing::{get_service, MethodRouter},
    Router,
};
use common::cfg::Cfg;
use tower_http::services::ServeDir;

use crate::account_handlers::account_routes;
use crate::auth_handlers::auth_routes;
use crate::heartbeat_handlers::heartbeat_handlers;
use crate::importance_settings_handlers::importance_settings_routes;
use crate::notify_settings_handlers::notify_settings_routes;


pub async fn init_server_instance(cfg: &Cfg) -> (axum::Router, std::net::SocketAddr) {
    // let assets_service = get_service(
    //     ServeDir::new(CONFIG.get::<String>("file_storage.path")));
    let static_service: MethodRouter = get_service(
        ServeDir::new(&cfg.web.static_path));
            
    let api_router = Router::new()
        .merge(heartbeat_handlers())
        .merge(account_routes())
        .merge(notify_settings_routes())
        .merge(importance_settings_routes());

    let router = Router::new()
        .merge(auth_routes())
        .nest("/api", api_router)
        // .route("/assets", assets_service)
        .fallback_service(static_service);

    (
        router,
        cfg.web.address.clone(),
    )
}
