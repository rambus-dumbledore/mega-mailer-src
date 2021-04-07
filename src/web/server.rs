use rocket_contrib::serve::StaticFiles;
use rocket::{get, routes, response::Redirect};
use rocket;

use crate::web::*;
use crate::cfg::CONFIG;

#[get("/")]
fn index() -> Redirect {
    Redirect::found("/static/index.html")
}

pub async fn init_server_instance() -> rocket::Rocket {
    let figment = rocket::Config::figment()
        .merge(("address", CONFIG.get::<String>("web.address")))
        .merge(("port", CONFIG.get::<u32>("web.port")))
    ;
    rocket::custom(figment)
        .mount("/api", account_routes())
        .mount("/", auth_routes())
        .mount("/assets", StaticFiles::from(CONFIG.get::<String>("file_storage.path")))
        .mount("/static", StaticFiles::from(CONFIG.get::<String>("web.static_path")))
        .mount("/", routes![index])
}
