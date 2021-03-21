use thiserror::Error;
use redis::{RedisError};
use rocket::response::Responder;
use serde_json::json;
use std::io::Cursor;
use rocket;
use serde_cbor;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("User is not registered")]
    UserNotRegistered,
    #[error("Authorization code is invalid")]
    AuthCodeInvalid,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage error: {0}")]
    StorageError(RedisError),
    #[error("Serialization error: {0}")]
    SerializationError(serde_cbor::Error),
    #[error("Account error: {0}")]
    AuthorizationError(AuthError),
}

impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let response = json!({
            "error": format!("{}", self)
        }).to_string();

        rocket::response::Response::build()
            .sized_body(response.len(), Cursor::new(response))
            .header(rocket::http::ContentType::JSON)
            .status(rocket::http::Status::InternalServerError)
            .ok()
    }
}

impl std::convert::From<RedisError> for Error {
    fn from(redis_error: RedisError) -> Self {
        Error::StorageError(redis_error)
    }
}

impl std::convert::From<serde_cbor::Error> for Error {
    fn from(serde_error: serde_cbor::Error) -> Self {
        Error::SerializationError(serde_error)
    }
}




