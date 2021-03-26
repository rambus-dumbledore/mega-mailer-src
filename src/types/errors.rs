use thiserror::Error;
use redis::{RedisError};
use rocket::response::Responder;
use serde_json::json;
use std::io::Cursor;
use rocket;
use serde_cbor;
use rustls_connector;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("User is not registered")]
    UserNotRegistered,
    #[error("Authorization code is invalid")]
    AuthCodeInvalid,
    #[error("User with this username is already registered")]
    UsernameAlreadyRegistered,
    #[error("Empty username")]
    UsernameEmpty,
}

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Handshake error: {0}")]
    HandshakeError(rustls_connector::HandshakeError<std::net::TcpStream>)
}

#[derive(Error, Debug)]
pub enum MailCheckerError {
    #[error("Empty envelope")]
    EmptyEnvelope,
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Redis connection error: {0}")]
    ConnectionError(RedisError),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage error: {0}")]
    StorageError(StorageError),
    #[error("Serialization error: {0}")]
    SerializationError(serde_cbor::Error),
    #[error("Account error: {0}")]
    AuthorizationError(AuthError),
    #[error("Parse integer error: {0}")]
    ParseIntError(std::num::ParseIntError),
    #[error("Telegram bot error: {0}")]
    TelegramBotError(telegram_bot::Error),
    #[error("Input/Output error: {0}")]
    IoError(std::io::Error),
    #[error("Network error: {0}")]
    NetworkError(NetworkError),
    #[error("Schedule error: {0}")]
    ScheduleError(schedule::error::Error),
    #[error("MailChecker error: {0}")]
    MailCheckerError(MailCheckerError),
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
        Error::StorageError(StorageError::ConnectionError(redis_error))
    }
}

impl std::convert::From<serde_cbor::Error> for Error {
    fn from(serde_error: serde_cbor::Error) -> Self {
        Error::SerializationError(serde_error)
    }
}

impl std::convert::From<std::num::ParseIntError> for Error {
    fn from(parse_int_error: std::num::ParseIntError) -> Self {
        Error::ParseIntError(parse_int_error)
    }
}

impl std::convert::From<telegram_bot::Error> for Error {
    fn from(tg_bot_error: telegram_bot::Error) -> Self {
        Error::TelegramBotError(tg_bot_error)
    }
}

impl std::convert::From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Error::IoError(io_error)
    }
}

impl std::convert::From<rustls_connector::HandshakeError<std::net::TcpStream>> for Error {
    fn from(hs_error: rustls_connector::HandshakeError<std::net::TcpStream>) -> Self {
        Error::NetworkError(NetworkError::HandshakeError(hs_error))
    }
}

impl std::convert::From<schedule::error::Error> for Error {
    fn from(schedule_error: schedule::error::Error) -> Self {
        Error::ScheduleError(schedule_error)
    }
}
