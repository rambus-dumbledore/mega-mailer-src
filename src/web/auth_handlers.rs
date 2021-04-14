use rocket::{get, post, routes, Route, State};
use rocket_contrib::json::Json;
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

#[post("/login", data = "<params>")]
fn login(mut sm: SessionManager<'_>, params: Json<LoginParams>) -> Result<()> {
    sm.authenticate(&params.username, &params.code)
}

#[derive(Deserialize)]
struct CodeParams {
    pub username: String,
}

#[post("/login_code", data = "<params>")]
async fn login_code(storage: State<'_, Arc<Storage>>, params: Json<CodeParams>) -> Result<()> {
    if params.username.len() == 0 {
        return Err(Error::AuthorizationError(AuthError::UsernameEmpty));
    }

    if let Ok(_id) = storage.get_telegram_id(&params.username) {
        let code = storage.create_login_request(&params.username);
        let text = format!("Your login code: {}", code);
        let to = params.username.clone();

        storage.add_send_message_task_to_queue(TelegramMessageTask { to, text, send_after: chrono::Utc::now() })?;
    } else {
        return Err(Error::AuthorizationError(AuthError::UserNotRegistered));
    }

    Ok(())
}

#[derive(Deserialize)]
struct AttachCodeParams {
    pub username: String,
}

#[post("/attach_code", data = "<params>")]
fn attach_code(storage: State<'_, Arc<Storage>>, params: Json<AttachCodeParams>) -> Result<String> {
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

#[get("/whoami")]
fn whoami(user: User) -> Json<User> {
    Json(user)
}

#[get("/logout")]
fn logout(mut sm: SessionManager) {
    sm.logout();
}

pub fn auth_routes() -> Vec<Route> {
    routes![login, attach_code, login_code, logout, whoami]
}
