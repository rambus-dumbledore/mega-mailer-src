use std::sync::Arc;
use axum::{extract::FromRequestParts, http::request::Parts};
use tower_cookies::{Cookie, Cookies, cookie::SameSite};

use crate::storage::Storage;
use crate::types::*;
use super::WebAppUser;

const COOKIE_NAME: &str = "mega_mailer_secret";

pub struct SessionManager {
    cookies: Cookies,
    storage: Arc<Storage>,
}

impl SessionManager {
    pub fn new(
        cookies: Cookies,
        storage: Arc<Storage>,
    ) -> SessionManager {
        SessionManager {
            cookies,
            storage,
        }
    }

    pub async fn auth_v2(&mut self, user: &WebAppUser) -> Result<()> {
        let cookie = uuid::Uuid::new_v4().to_string();
        self.storage.set_session_v2(&cookie, &user).await?;

        if !self.storage.is_user_registed(user).await? {
            self.storage.register_user(user).await?;
        }

        self.cookies.add(
            Cookie::build((COOKIE_NAME, cookie))
            .same_site(SameSite::Lax)
            .build()
        );

        Ok(())
    }

    pub async fn is_authorized_v2(&mut self) -> Result<bool> {
        let cookie = match self.get_cookie() {
            Some(cookie) => cookie,
            None => return Ok(false)
        };
        match self.storage.get_session_v2(&cookie).await? {
            Some(_) => Ok(true),
            None => Ok(false)
        }
    }

    fn get_cookie(&self) -> Option<String> {
        let cookie = self.cookies.get(COOKIE_NAME);
        let cookie = match cookie {
            Some(cookie) => {
                cookie
            },
            None => return None
        };
        Some(cookie.value().to_owned())
    }

    pub async fn get_user_v2(&self) -> Result<WebAppUser> {
        let cookie = match self.get_cookie() {
            Some(cookie) => cookie,
            None => return Err(Error::InternalError(InternalError::RuntimeError(format!("Unauthorized"))))
        };
        match self.storage.get_session_v2(&cookie).await? {
            Some(user) => Ok(user),
            None => Err(Error::InternalError(InternalError::RuntimeError(format!("Unauthorized"))))
        }
    }

    pub async fn logout_v2(&mut self) {
        let ref cookie = match self.get_cookie() {
            Some(cookie) => cookie,
            None => return
        };
        self.storage.remove_session_v2(cookie).await.ok();
        self.cookies.remove(Cookie::build(COOKIE_NAME).build());
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for SessionManager
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(req: &mut Parts, _state: &S) -> std::result::Result<Self, Self::Rejection> {
        let cookies = req.extensions.get::<Cookies>().cloned().unwrap();
        let storage = req
            .extensions
            .get::<Arc<Storage>>()
            .cloned()
            .unwrap();
        Ok(SessionManager {
            cookies,
            storage,
        })
    }
}
