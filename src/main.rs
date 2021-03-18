mod storage;
mod web;
mod types;
mod bot;
mod cfg;

use storage::Storage;
use web::{SessionKeystore};
use cfg::CONFIG;

use pretty_env_logger;

fn main() {
    pretty_env_logger::init();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        let session_keystore = SessionKeystore::new();
        let storage = Storage::new();
        let bot = bot::TelegramBot::new(&CONFIG.get::<String>("bot.secret"), storage.clone());

        let instance = web::init_server_instance()
            .await
            .manage(storage)
            .manage(session_keystore)
            .manage(bot);

        instance.launch()
            .await
            .unwrap();
    })
}
