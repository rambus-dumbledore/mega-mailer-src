use serde::Deserialize;
use rocket::{State, get, post, Route, routes};
use serde_json::json;
use rocket_contrib::json::Json;
use std::sync::Arc;

use crate::storage::{User, Storage};
use crate::web::session_manager::{SessionManager};
use crate::bot::TelegramBot;
use crate::types::{Result, AuthError, Error};

#[derive(Deserialize)]
struct LoginParams {
    pub username: String,
    pub code: String
}

#[post("/login", data = "<params>")]
fn login(mut sm: SessionManager<'_>, params: Json<LoginParams>) -> Result<()> {
    sm.authenticate(&params.username, &params.code)
}

#[derive(Deserialize)]
struct CodeParams {
    pub username: String
}

#[post("/login_code", data = "<params>")]
async fn login_code(storage: State<'_, Arc<Storage>>, bot: State<'_, TelegramBot>, params: Json<CodeParams>) -> Result<()> {
    if params.username.len() == 0 {
        return Err(Error::AuthorizationError(AuthError::UsernameEmpty))
    }

    if let Ok(id) = storage.get_telegram_id(&params.username) {
        bot.send_login_code(id, &params.username).await?;
    } else {
        return Err(Error::AuthorizationError(AuthError::UserNotRegistered))
    }

    Ok(())
}

#[derive(Deserialize)]
struct AttachCodeParams {
    pub username: String
}

#[post("/attach_code", data = "<params>")]
fn attach_code(storage: State<'_, Arc<Storage>>, params: Json<AttachCodeParams>) -> Result<String> {
    if params.username.len() == 0 {
        return Err(Error::AuthorizationError(AuthError::UsernameEmpty))
    }

    if let Ok(_) = storage.get_telegram_id(&params.username) {
        return Err(Error::AuthorizationError(AuthError::UsernameAlreadyRegistered))
    }

    let code = storage.create_attach_request(&params.username)?;
    Ok(json!({
        "code": code
    }).to_string())
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
