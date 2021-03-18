use rocket_contrib::serve::StaticFiles;
use rocket;
use crate::web::*;
use crate::cfg::CONFIG;

pub async fn init_server_instance() -> rocket::Rocket {
    let figment = rocket::Config::figment()
        .merge(("address", CONFIG.get::<String>("web.address")))
        .merge(("port", CONFIG.get::<u32>("web.port")))
    ;
    rocket::custom(figment)
        .mount("/api", account_routes())
        .mount("/", auth_routes())
        .mount("/", StaticFiles::from(CONFIG.get::<String>("web.static_path")))
}