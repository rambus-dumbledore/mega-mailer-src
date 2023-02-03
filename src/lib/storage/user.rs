use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use serde::{Deserialize, Serialize};

use crate::sessions::SessionManager;

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub photo: Option<String>,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(req: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let sm = req.extensions
            .get_mut::<SessionManager>()
            .unwrap();
        if sm.is_authorized() {
            Ok(sm.get_user().unwrap())
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
