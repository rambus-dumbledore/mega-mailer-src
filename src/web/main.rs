#![feature(type_alias_impl_trait)]
#![feature(backtrace)]

mod account_handlers;
mod auth_handlers;
mod server;
mod notify_settings_handlers;
mod importance_settings_handlers;

use log::error;
use pretty_env_logger;
use std::sync::Arc;

use common::sessions::SessionKeystore;
use common::storage::Storage;

use server::init_server_instance;

fn main() {
    pretty_env_logger::init();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        let session_keystore = SessionKeystore::new();

        let storage = Storage::new();
        if let Err(err) = storage {
            error!("Could not create connection to storage: {}", err);
            return;
        }
        let storage = Arc::new(storage.unwrap());

        let instance = init_server_instance()
            .await
            .manage(storage)
            .manage(session_keystore);

        instance.launch().await.unwrap();
    })
}
