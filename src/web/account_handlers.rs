use rocket::{get, post, routes, Route, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use common::storage::{MailAccount, Storage, User};
use common::types::Result;
use rocket_contrib::json::Json;

#[get("/account")]
fn get_account_settings(user: User, storage: State<Arc<Storage>>) -> Json<Option<MailAccount>> {
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

#[post("/account", data = "<params>")]
fn set_account_settings(
    user: User,
    params: Json<SetAccountParams>,
    storage: State<Arc<Storage>>,
) -> Result<Json<SetAccountResponse>> {
    let changed = storage.set_mail_account(&user.username, &params.email, &params.password)?;
    Ok(Json(SetAccountResponse { changed }))
}

#[get("/checking")]
fn get_checking_state(user: User, storage: State<Arc<Storage>>) -> Result<Json<bool>> {
    let res = storage.is_checking_enabled(&user.username)?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct SetCheckingParams {
    state: bool,
}

#[post("/checking", data = "<params>")]
fn set_checking(
    user: User,
    storage: State<Arc<Storage>>,
    params: Json<SetCheckingParams>,
) -> Result<()> {
    if params.state {
        let _ = storage.enable_checking(&user.username)?;
    } else {
        let _ = storage.disable_checking(&user.username)?;
    }
    Ok(())
}

pub fn account_routes() -> Vec<Route> {
    routes![
        get_account_settings,
        set_account_settings,
        get_checking_state,
        set_checking
    ]
}
