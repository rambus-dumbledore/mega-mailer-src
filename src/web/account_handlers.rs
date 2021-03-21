use serde::{Serialize, Deserialize};
use rocket::{State, get, post, routes, Route};
use crate::storage::{User, Storage, MailAccount};
use rocket_contrib::json::Json;

#[get("/account")]
fn get_account_settings(user: User, storage: State<Storage>) -> Json<Option<MailAccount>>{
    let account = storage.get_mail_account(&user.user_name);
    Json(account)
}

#[derive(Deserialize)]
struct SetAccountParams {
    pub email: String,
    pub password: String
}

#[derive(Serialize)]
struct SetAccountResponse {
    changed: bool,
}

#[post("/account", data = "<params>")]
fn set_account_settings(user: User, params: Json<SetAccountParams>, storage: State<Storage>) -> Result<Json<SetAccountResponse>> {
    let changed = storage.set_mail_account(&user.user_name, &params.email, &params.password)?;
    Ok(Json(SetAccountResponse{ changed }))
}

pub fn account_routes() -> Vec<Route> {
    routes![get_account_settings, set_account_settings]
}
