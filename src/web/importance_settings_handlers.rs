use axum::{
    extract::Extension, response::IntoResponse, routing::get, Json, Router,
};
use std::sync::Arc;
use common::sessions::WebAppUser;

use common::storage::Storage;
use common::types::Result;

async fn get_important_emails(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
) -> Result<impl IntoResponse> {
    let emails: Vec<String> = storage
        .get_important_emails(&user).await
        .unwrap_or(vec![]);
    Ok(Json(emails))
}

async fn get_important_tags(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
) -> Result<impl IntoResponse> {
    let tags: Vec<String> = storage
        .get_important_tags(&user).await
        .unwrap_or(vec![])
        .into();
    Ok(Json(tags))
}

async fn set_important_tags(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
    Json(tags): Json<Vec<String>>,
) -> Result<impl IntoResponse> {
    storage.set_important_tags(&user, &tags).await?;
    Ok(())
}

pub fn importance_settings_routes() -> Router {
    Router::new()
        .route(
            "/important_emails",
            get(get_important_emails)
        )
        .route(
            "/important_tags",
            get(get_important_tags)
                .post(set_important_tags),
        )
}
