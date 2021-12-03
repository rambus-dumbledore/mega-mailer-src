use axum::{routing::{get}, extract::Extension, Router, Json, extract::{Query}, response::IntoResponse};
use serde::Deserialize;

use common::storage::{Storage, User};
use common::types::Result;
use std::sync::Arc;

#[derive(Deserialize)]
struct Email {
    email: String,
}

async fn get_important_emails(user: User, Extension(storage): Extension<Arc<Storage>>) -> impl IntoResponse {
    let emails: Vec<String> = storage.get_important_emails(&user.username).unwrap_or(vec![]).into();
    Json(emails)
}

async fn add_important_email(user: User, Extension(storage): Extension<Arc<Storage>>, Query(query): Query<Email>) -> Result<impl IntoResponse> {
    storage.add_important_email(&user.username, &query.email)?;
    Ok(())
}

async fn remove_important_email(user: User, Extension(storage): Extension<Arc<Storage>>, Query(query): Query<Email>) -> Result<impl IntoResponse> {
    storage.remove_important_email(&user.username, &query.email)?;
    Ok(())
}

#[derive(Deserialize)]
struct Tag {
    tag: String,
}


async fn get_important_tags(user: User, Extension(storage): Extension<Arc<Storage>>) -> impl IntoResponse {
   let tags: Vec<String> = storage.get_important_tags(&user.username).unwrap_or(vec![]).into();
   Json(tags)
}

async fn add_important_tag(user: User, Extension(storage): Extension<Arc<Storage>>, Query(query): Query<Tag>) -> Result<impl IntoResponse> {
    storage.add_important_tag(&user.username, &query.tag)?;
    Ok(())
}

async fn remove_important_tag(user: User, Extension(storage): Extension<Arc<Storage>>, Query(query): Query<Tag>) -> Result<impl IntoResponse> {
    storage.remove_important_tag(&user.username, &query.tag)?;
    Ok(())
}

pub fn importance_settings_routes() -> Router {
    Router::new()
        .route("/important_emails", get(get_important_emails).patch(add_important_email).delete(remove_important_email))
        .route("/important_tags", get(get_important_tags).patch(add_important_tag).delete(remove_important_tag))
}
