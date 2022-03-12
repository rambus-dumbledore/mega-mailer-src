use axum::{
    extract::{FromRequest, RequestParts},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_cookies::Cookies;

use crate::sessions::{SessionKeystore, SessionManager};
use crate::storage::Storage;

#[derive(Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub photo: Option<String>,
}

#[axum::async_trait]
impl<B> FromRequest<B> for User
where
    B: Send,
{
    type Rejection = (axum::http::StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let keystore = req
            .extensions()
            .unwrap()
            .get::<SessionKeystore>()
            .cloned()
            .unwrap();
        let cookies = req.extensions().unwrap().get::<Cookies>().cloned().unwrap();
        let storage = req
            .extensions()
            .unwrap()
            .get::<Arc<Storage>>()
            .cloned()
            .unwrap();
        let mut sm = SessionManager::new(cookies, keystore, storage);
        if sm.is_authorized() {
            Ok(sm.get_user().unwrap())
        } else {
            Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
        }
    }
}
