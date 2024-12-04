use axum::{response::IntoResponse, routing::get, Router};

use common::types::Result;

async fn healthcheck() -> Result<impl IntoResponse> {
    Ok(())
}

pub fn heartbeat_handlers() -> Router {
    Router::new().route("/healthcheck", get(healthcheck))
}
