use std::sync::Arc;
use axum::{extract::Extension, routing::get, Json, Router};

use common::{
    storage::Storage,
    types::Result, sessions::WebAppUser,
};

async fn get_working_hours(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
) -> Result<Json<[u8; 2]>> {
    let res = storage.get_user_working_hours(&user).await?;
    Ok(Json(res))
}

async fn set_working_hours(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
    Json(wh): Json<[u8; 2]>,
) -> Result<()> {
    storage.set_user_working_hours(&user, &wh).await?;
    Ok(())
}

pub fn notify_settings_routes() -> Router {
    Router::new().route(
        "/working_hours",
        get(get_working_hours).post(set_working_hours),
    )
}
