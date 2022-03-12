use axum::{extract::Extension, response::IntoResponse, routing::get, Json, Router};
use std::sync::Arc;

use common::storage::Storage;
use common::types::Result;

async fn heartbeat(Extension(storage): Extension<Arc<Storage>>) -> Result<impl IntoResponse> {
    Ok(Json(storage.get_heartbeat()?))
}

pub fn heartbeat_handlers() -> Router {
    Router::new().route("/heartbeat", get(heartbeat))
}
