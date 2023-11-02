use serde::{Serialize, Deserialize};
use crate::sessions::SessionManager;
use anyhow::anyhow;
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

#[derive(Debug, Deserialize)]
pub struct WebAppInitData {
    pub query_id: Option<String>,
    pub user: Option<WebAppUser>,
    pub hash: String,

    pub data_check_string: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, Hash)]
pub struct WebAppUser {
    pub id: i64,
}

impl std::convert::From<i64> for WebAppUser {
    fn from(id: i64) -> Self {
        Self{ id }
    }
}

impl std::cmp::PartialEq for WebAppUser {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl TryFrom<&str> for WebAppInitData {
    type Error = String;

    fn try_from(query: &str) -> Result<Self, Self::Error> {
        let params = querystring::querify(&query);
        let mut items = query.split("&")
            .filter(|s| !s.starts_with("hash="))
            .map(|str| urlencoding::decode(str).unwrap().to_string())
            .collect::<Vec<String>>();
        items.sort();
        let mut init_data: Self = Self{
            data_check_string: items.join("\n"),
            hash: String::new(),
            query_id: None,
            user: None
        };
        for (key, value) in params {
            match key {
                "query_id" => {
                    init_data.query_id = Some(value.to_owned())
                },
                "hash" => {
                    init_data.hash = value.to_owned()
                },
                "user" => {
                    let decoded = urlencoding::decode(value)
                        .map_err(|e| format!("failed to url decode user value: {}", e))?;
                    let user = serde_json::from_str(&decoded)
                        .map_err(|e| format!("failed to user json decode: {}", e))?;
                    init_data.user = Some(user)
                },
                _ => {}
            }
        }

        if init_data.hash.is_empty() {
            return Err(format!("TryFrom failed: hash is empty"));
        }

        Ok(init_data)
    }
}

use ring::hmac;

impl WebAppInitData {
    pub fn validate(&self, bot_token: &String) -> anyhow::Result<()> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, "WebAppData".as_bytes());
        let secret_key = hmac::sign(&key, bot_token.as_bytes());

        let key2 = hmac::Key::new(hmac::HMAC_SHA256, secret_key.as_ref());
        let second_secret_key = hmac::sign(&key2, self.data_check_string.as_bytes());

        let hash = hex::encode(second_secret_key.as_ref());
        if hash != self.hash {
            return Err(anyhow!("WebAppInitData is not valid"));
        }
        Ok(())
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for WebAppUser
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(req: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let sm = req.extensions
            .get_mut::<SessionManager>()
            .unwrap();
        if sm.is_authorized_v2().await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)? {
            Ok(sm.get_user_v2().await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?)
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}