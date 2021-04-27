use common::storage::Storage;
use common::types::Result;
use rocket::{get, State, Route, routes};
use rocket_contrib::json::Json;
use std::collections::BTreeMap;
use std::sync::Arc;

#[get("/heartbeat")]
fn heartbeat(storage: State<Arc<Storage>>) -> Result<Json<BTreeMap<String, i64>>> {
    Ok(Json(storage.get_heartbeat()?))
}

pub fn heartbeat_handlers() -> Vec<Route> {
    routes![
        heartbeat
    ]
}
