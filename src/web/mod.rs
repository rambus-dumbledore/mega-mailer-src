mod account_handlers;
mod auth_handlers;
mod server;

pub use account_handlers::account_routes;
pub use auth_handlers::auth_routes;
pub use server::init_server_instance;
