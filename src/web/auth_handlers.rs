use serde::Deserialize;
use rocket::{State, get, post, Route, routes};
use crate::storage::{User, Storage};
use crate::web::session_manager::{SessionManager};
use rocket_contrib::json::Json;
use serde_json::json;
use crate::bot::TelegramBot;

#[derive(Deserialize)]
struct LoginParams {
    pub username: String,
    pub code: String
}

#[post("/login", data = "<params>")]
fn login(mut sm: SessionManager<'_>, params: Json<LoginParams>) {
    sm.authenticate(&params.username, &params.code).unwrap()
}

#[derive(Deserialize)]
struct CodeParams {
    pub username: String
}

#[post("/login_code", data = "<params>")]
async fn login_code(storage: State<'_, Storage>, bot: State<'_, TelegramBot>, params: Json<CodeParams>) {
    if let Some(id) = storage.get_telegram_id(&params.username) {
        bot.send_login_code(id, &params.username).await;
    }
}

#[derive(Deserialize)]
struct AttachCodeParams {
    pub username: String
}

#[post("/attach_code", data = "<params>")]
fn attach_code(storage: State<'_, Storage>, params: Json<AttachCodeParams>) -> String {
    json!({
        "code": storage.create_attach_request(&params.username)
    }).to_string()
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