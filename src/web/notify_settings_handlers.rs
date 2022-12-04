use axum::{extract::Extension, response::IntoResponse, routing::get, Json, Router};

use common::{
    storage::{Storage, User},
    types::Result,
};
use std::sync::Arc;

async fn get_working_hours(
    user: User,
    Extension(storage): Extension<Arc<Storage>>,
) -> Json<Option<Vec<u8>>> {
    let res = storage.get_user_working_hours(&user.username);
    Json(res)
}

async fn set_working_hours(
    user: User,
    Extension(storage): Extension<Arc<Storage>>,
    Json(params): Json<Vec<u8>>,
) -> Result<()> {
    storage.set_user_working_hours(&user.username, &params)?;
    Ok(())
}

pub fn notify_settings_routes() -> Router {
    Router::new().route(
        "/working_hours",
        get(get_working_hours).post(set_working_hours),
    )
}
