use common::cfg::CONFIG;
use rocket;
use rocket::{get, response::Redirect, routes, Build, Rocket};
use rocket_contrib::serve::StaticFiles;

use crate::account_handlers::account_routes;
use crate::auth_handlers::auth_routes;
use crate::notify_settings_handlers::notify_settings_routes;
use crate::importance_settings_handlers::importance_settings_routes;

#[get("/")]
fn index() -> Redirect {
    Redirect::found("/static/index.html")
}

pub async fn init_server_instance() -> Rocket<Build> {
    let figment = rocket::Config::figment()
        .merge(("address", CONFIG.get::<String>("web.address")))
        .merge(("port", CONFIG.get::<u32>("web.port")));
    rocket::custom(figment)
        .mount("/api", account_routes())
        .mount("/api", notify_settings_routes())
        .mount("/api", importance_settings_routes())
        .mount("/", auth_routes())
        .mount(
            "/assets",
            StaticFiles::from(CONFIG.get::<String>("file_storage.path")),
        )
        .mount(
            "/static",
            StaticFiles::from(CONFIG.get::<String>("web.static_path")),
        )
        .mount("/", routes![index])
}
