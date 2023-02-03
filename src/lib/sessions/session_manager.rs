use axum::{extract::FromRequestParts, http::request::Parts};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::sync::Arc;

use cookie::SameSite;
use tower_cookies::{Cookie, Cookies};

use crate::cfg::CONFIG;
use crate::storage::{Storage, User};
use crate::types::*;

type HmacSha256 = Hmac<Sha256>;

const COOKIE_NAME: &str = "mega_mailer_secret";

#[derive(Clone)]
pub struct SessionKeystore {
    pub key: Hmac<Sha256>,
}

impl SessionKeystore {
    pub fn new() -> SessionKeystore {
        SessionKeystore {
            key: HmacSha256::new_from_slice(CONFIG.get::<String>("web.cookie_key").as_bytes())
                .unwrap(),
        }
    }
}

pub struct SessionManager {
    cookies: Cookies,
    keystore: SessionKeystore,
    storage: Arc<Storage>,
}

impl SessionManager {
    pub fn new(
        cookies: Cookies,
        keystore: SessionKeystore,
        storage: Arc<Storage>,
    ) -> SessionManager {
        SessionManager {
            cookies,
            keystore,
            storage,
        }
    }

    pub fn authenticate(&mut self, user_name: &String, code: &String) -> Result<()> {
        let telegram_id = self.storage.get_telegram_id(user_name);
        if telegram_id.is_err() {
            return Err(Error::AuthorizationError(AuthError::UserNotRegistered));
        }

        let login_request = self.storage.get_login_request(code);
        if login_request.is_none() {
            return Err(Error::AuthorizationError(AuthError::AuthCodeInvalid));
        }

        let mut claims = BTreeMap::new();
        claims.insert("username", user_name);
        let cookie = SignWithKey::sign_with_key(claims, &self.keystore.key).unwrap();

        let user = User {
            username: user_name.clone(),
            photo: None,
        };

        self.storage.set_session(&user)?;
        self.cookies.add(
            Cookie::build(COOKIE_NAME, cookie)
                .same_site(SameSite::Lax)
                .finish(),
        );

        Ok(())
    }

    fn get_tree(&mut self) -> Option<BTreeMap<String, String>> {
        let cookie = self.cookies.get(COOKIE_NAME);
        if let Some(cookie) = cookie {
            let token = cookie.value();
            let result = VerifyWithKey::<BTreeMap<String, String>>::verify_with_key(
                token,
                &self.keystore.key,
            );
            if let Ok(tree) = result {
                return Some(tree);
            }
            return None;
        }
        return None;
    }

    pub fn is_authorized(&mut self) -> bool {
        if let Some(_) = self.get_tree() {
            return true;
        }
        return false;
    }

    pub fn get_user(&mut self) -> Option<User> {
        if let Some(tree) = self.get_tree() {
            let username = tree.get("username").unwrap().clone();
            let user = self.storage.get_session(&username);

            if let Some(mut user) = user {
                if let Ok(photo) = self.storage.get_user_avatar(&username) {
                    user.photo = Some(format!("/assets/{}", photo));
                }
                return Some(user);
            }
            return None;
        }
        None
    }

    pub fn logout(&mut self) {
        let tree = self.get_tree();
        match tree {
            Some(tree) => {
                let username = tree.get("username").unwrap().clone();
                self.storage.remove_session(&username);
                self.cookies.remove(Cookie::named(COOKIE_NAME));
            }
            _ => {}
        }
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for SessionManager
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(req: &mut Parts, _state: &S) -> std::result::Result<Self, Self::Rejection> {
        let keystore = req
            .extensions
            .get::<SessionKeystore>()
            .cloned()
            .unwrap();
        let cookies = req.extensions.get::<Cookies>().cloned().unwrap();
        let storage = req
            .extensions
            .get::<Arc<Storage>>()
            .cloned()
            .unwrap();
        Ok(SessionManager {
            keystore,
            cookies,
            storage,
        })
    }
}
