use axum::{Json, Router, extract::Extension, response::IntoResponse, routing::{get}};

use common::{storage::{Storage, User}, types::Result};
use std::sync::Arc;

async fn get_working_hours(user: User, Extension(storage): Extension<Arc<Storage>>) -> impl IntoResponse {
    let res = storage.get_user_working_hours(&user.username);
    Json(res)
}

async fn set_working_hours(user: User, Extension(storage): Extension<Arc<Storage>>, Json(params): Json<Vec<u8>>) -> Result<impl IntoResponse> {
    storage.set_user_working_hours(&user.username, &params)?;
    Ok(())
}

pub fn notify_settings_routes() -> Router {
    Router::new()
        .route("/working_hours", get(get_working_hours).post(set_working_hours))
}
