use axum::{Router, extract::Extension, routing::{get}, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use common::{storage::{MailAccount, Storage, User}, types::Result};

async fn get_account_settings(user: User, Extension(storage): Extension<Arc<Storage>>) -> Json<Option<MailAccount>> {
    let account = storage.get_mail_account(&user.username);
    Json(account)
}

#[derive(Deserialize)]
struct SetAccountParams {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
struct SetAccountResponse {
    changed: bool,
}

async fn set_account_settings(
    user: User,
    params: Json<SetAccountParams>,
    Extension(storage): Extension<Arc<Storage>>,
) -> Result<impl IntoResponse> {
    let changed = storage.set_mail_account(&user.username, &params.email, &params.password)?;
    Ok(Json(SetAccountResponse { changed }))
}

async fn get_checking_state(user: User, Extension(storage): Extension<Arc<Storage>>) -> Result<impl IntoResponse> {
    let res = storage.is_checking_enabled(&user.username)?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct SetCheckingParams {
    state: bool,
}

async fn set_checking(
    user: User,
    Extension(storage): Extension<Arc<Storage>>,
    params: Json<SetCheckingParams>,
) -> Result<impl IntoResponse> {
    if params.state {
        let _ = storage.enable_checking(&user.username)?;
    } else {
        let _ = storage.disable_checking(&user.username)?;
    }
    Ok(())
}

pub fn account_routes() -> Router {
    Router::new()
        .route("/account", get(get_account_settings).post(set_account_settings))
        .route("/checking", get(get_checking_state).post(set_checking))
}
