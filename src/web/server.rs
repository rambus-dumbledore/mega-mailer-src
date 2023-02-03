use axum::{
    http::StatusCode,
    routing::{get_service, MethodRouter},
    Router,
};
use common::cfg::CONFIG;
use tower_http::services::ServeDir;

use crate::account_handlers::account_routes;
use crate::auth_handlers::auth_routes;
use crate::heartbeat_handlers::heartbeat_handlers;
use crate::importance_settings_handlers::importance_settings_routes;
use crate::notify_settings_handlers::notify_settings_routes;


pub async fn init_server_instance() -> (axum::Router, String, u16) {
    let assets_service = get_service(
        ServeDir::new(CONFIG.get::<String>("file_storage.path")))
            .handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            });
    let static_service: MethodRouter = get_service(
        ServeDir::new(CONFIG.get::<String>("web.static_path")))
            .handle_error(|error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            });
    let router = Router::new()
        .merge(auth_routes())
        .nest("/api", heartbeat_handlers())
        .nest("/api", account_routes())
        .nest("/api", notify_settings_routes())
        .nest("/api", importance_settings_routes())
        .route("/assets", assets_service)
        .fallback_service(static_service);

    (
        router,
        CONFIG.get::<String>("web.address"),
        CONFIG.get::<u16>("web.port"),
    )
}
