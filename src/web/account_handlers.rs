use serde::Deserialize;
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

#[post("/account", data = "<params>")]
fn set_account_settings(user: User, params: Json<SetAccountParams>, storage: State<Storage>) {
    storage.set_mail_account(&user.user_name, &params.email, &params.password);
}

pub fn account_routes() -> Vec<Route> {
    routes![get_account_settings, set_account_settings]
}
