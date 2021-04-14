use rocket::{get, post, routes, Route, State};
use rocket_contrib::json::Json;

use common::storage::{Storage, User};
use common::types::Result;
use std::sync::Arc;

#[get("/working_hours")]
fn get_working_hours(user: User, storage: State<Arc<Storage>>) -> Json<Option<Vec<u8>>> {
    storage.get_user_working_hours(&user.username).into()
}

#[post("/working_hours", data = "<params>")]
fn set_working_hours(user: User, storage: State<Arc<Storage>>, params: Json<Vec<u8>>) -> Result<()> {
    storage.set_user_working_hours(&user.username, &params)?;
    Ok(())
}

pub fn notify_settings_routes() -> Vec<Route> {
    routes![
        get_working_hours,
        set_working_hours,
    ]
}
