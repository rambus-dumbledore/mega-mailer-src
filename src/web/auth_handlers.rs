use axum::{
    extract::Extension,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use common::sessions::SessionManager;
use common::storage::{Storage, User};
use common::types::{AuthError, Error, Result, TelegramMessageTask};

#[derive(Deserialize)]
struct LoginParams {
    pub username: String,
    pub code: String,
}

async fn login(
    mut sm: SessionManager,
    Json(params): Json<LoginParams>,
) -> impl IntoResponse {
    sm.authenticate(&params.username, &params.code)
}

#[derive(Deserialize)]
struct CodeParams {
    pub username: String,
}

async fn login_code(
    Extension(storage): Extension<Arc<Storage>>,
    Json(params): Json<CodeParams>,
) -> Result<impl IntoResponse> {
    if params.username.len() == 0 {
        return Err(Error::AuthorizationError(AuthError::UsernameEmpty));
    }

    if let Ok(_id) = storage.get_telegram_id(&params.username) {
        let code = storage.create_login_request(&params.username);
        let text = format!("Your login code: {}", code);
        let to = params.username.clone();

        storage.add_send_message_task_to_queue(TelegramMessageTask {
            to,
            text,
            send_after: chrono::Utc::now(),
            important: true,
        })?;
    } else {
        return Err(Error::AuthorizationError(AuthError::UserNotRegistered));
    }

    Ok(())
}

#[derive(Deserialize)]
struct AttachCodeParams {
    pub username: String,
}

async fn attach_code(
    Extension(storage): Extension<Arc<Storage>>,
    Json(params): Json<AttachCodeParams>,
) -> Result<impl IntoResponse> {
    if params.username.len() == 0 {
        return Err(Error::AuthorizationError(AuthError::UsernameEmpty));
    }

    if let Ok(_) = storage.get_telegram_id(&params.username) {
        return Err(Error::AuthorizationError(
            AuthError::UsernameAlreadyRegistered,
        ));
    }

    let code = storage.create_attach_request(&params.username)?;
    Ok(json!({ "code": code }).to_string())
}

async fn whoami(user: User) -> impl IntoResponse {
    Json(user)
}

async fn logout(mut sm: SessionManager) -> impl IntoResponse {
    sm.logout();
}

pub fn auth_routes() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/login_code", post(login_code))
        .route("/attach_code", post(attach_code))
        .route("/whoami", get(whoami))
        .route("/logout", get(logout))
}
