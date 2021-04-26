use rocket::{get, patch, delete, routes, Route, State};
use rocket_contrib::json::Json;

use common::storage::{Storage, User};
use common::types::Result;
use std::sync::Arc;

#[get("/important_emails")]
fn get_important_emails(user: User, storage: State<Arc<Storage>>) -> Json<Vec<String>> {
    storage.get_important_emails(&user.username).unwrap_or(vec![]).into()
}

#[patch("/important_emails?<email>")]
fn add_important_email(user: User, storage: State<Arc<Storage>>, email: String) -> Result<()> {
    storage.add_important_email(&user.username, &email)?;
    Ok(())
}

#[delete("/important_emails?<email>")]
fn remove_important_email(user: User, storage: State<Arc<Storage>>, email: String) -> Result<()> {
    storage.remove_important_email(&user.username, &email)?;
    Ok(())
}

#[get("/important_tags")]
fn get_important_tags(user: User, storage: State<Arc<Storage>>) -> Json<Vec<String>> {
    storage.get_important_tags(&user.username).unwrap_or(vec![]).into()
}

#[patch("/important_tags?<tag>")]
fn add_important_tag(user: User, storage: State<Arc<Storage>>, tag: String) -> Result<()> {
    storage.add_important_tag(&user.username, &tag)?;
    Ok(())
}

#[delete("/important_tags?<tag>")]
fn remove_important_tag(user: User, storage: State<Arc<Storage>>, tag: String) -> Result<()> {
    storage.remove_important_tag(&user.username, &tag)?;
    Ok(())
}

pub fn importance_settings_routes() -> Vec<Route> {
    routes![
        get_important_emails,
        add_important_email,
        remove_important_email,
        get_important_tags,
        add_important_tag,
        remove_important_tag,
    ]
}
