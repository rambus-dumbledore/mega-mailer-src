use axum::{extract::Extension, response::IntoResponse, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use common::{
    storage::{Storage, Cipher},
    types::Result, sessions::WebAppUser,
};

async fn get_account_settings(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
    Extension(cipher): Extension<Arc<Cipher>>
) -> Result<impl IntoResponse> {
    let account = storage.get_mail_account(&user, &cipher).await?;
    Ok(Json(account))
}

#[derive(Deserialize)]
struct SetAccountParams {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Debug)]
struct SetAccountResponse {
    changed: bool,
}

async fn set_account_settings(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
    Extension(cipher): Extension<Arc<Cipher>>,
    Json(params): Json<SetAccountParams>,
) -> Result<impl IntoResponse> {
    storage.set_mail_account(&user, &params.email, &params.password, &cipher).await?;
    Ok(Json(SetAccountResponse { changed: true }))
}

async fn get_checking_state(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
) -> Result<Json<bool>> {
    let res = storage.is_checking_enabled(&user).await?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct SetCheckingParams {
    state: bool,
}

async fn set_checking(
    user: WebAppUser,
    Extension(storage): Extension<Arc<Storage>>,
    Json(params): Json<SetCheckingParams>,
) -> Result<impl IntoResponse> {
    if params.state {
        let _ = storage.enable_checking(&user).await?;
    } else {
        let _ = storage.disable_checking(&user).await?;
    }
    Ok(())
}

pub fn account_routes() -> Router {
    Router::new()
        .route(
            "/account",
            get(get_account_settings).post(set_account_settings),
        )
        .route("/checking", get(get_checking_state).post(set_checking))
}
