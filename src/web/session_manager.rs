use hmac::{ Hmac, NewMac };
use jwt::{SignWithKey, VerifyWithKey};
use sha2::Sha256;
use std::collections::BTreeMap;

use rocket::http::{Cookie, SameSite, CookieJar};
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, State};

use crate::storage::{User, Storage};
use crate::types::*;
use crate::cfg::CONFIG;

const COOKIE_NAME: &str = "mega_mailer_secret";

pub struct SessionKeystore {
    pub key: Hmac<Sha256>
}

impl SessionKeystore {
    pub fn new() -> SessionKeystore {
        SessionKeystore {
            key: Hmac::new_varkey(CONFIG.get::<String>("web.cookie_key").as_bytes()).unwrap()
        }
    }
}

pub struct SessionManager<'a> {
    cookies: &'a CookieJar<'a>,
    keystore: State<'a, SessionKeystore>,
    storage: State<'a, Storage>
}

impl SessionManager<'_> {
    pub fn new<'a>(cookies: &'a CookieJar<'a>, keystore: State<'a, SessionKeystore>, storage: State<'a, Storage>) -> SessionManager<'a> {
        SessionManager{
            cookies,
            keystore,
            storage
        }
    }

    pub fn authenticate(&mut self, user_name: &String, code: &String) -> Result<()> {
        let telegram_id = self.storage.get_telegram_id(user_name);
        if telegram_id.is_err() {
            return Err(Error::AuthorizationError(AuthError::UserNotRegistered));
        }

        let login_request = self.storage.get_login_request(code);
        if login_request.is_none() {
            return Err(Error::AuthorizationError(AuthError::AuthCodeInvalid))
        }

        let mut claims = BTreeMap::new();
        claims.insert("username", user_name);
        let cookie = SignWithKey::sign_with_key(claims, &self.keystore.key).unwrap();

        let user = User{
            username: user_name.clone(),
            photo: None
        };

        self.storage.set_session(&user)?;
        self.cookies.add(
            Cookie::build(COOKIE_NAME, cookie)
                .same_site(SameSite::Lax)
                .finish()
        );

        Ok(())
    }

    fn get_tree(&mut self) -> Option<BTreeMap<String, String>> {
        let cookie = self.cookies.get(COOKIE_NAME);
        if let Some(cookie) = cookie {
            let token = cookie.value();
            let result = VerifyWithKey::<BTreeMap<String, String>>::verify_with_key(token, &self.keystore.key);
            if let Ok(tree) = result {
                return Some(tree)
            }
            return None
        }
        return None
    }

    pub fn is_authorized(&mut self) -> bool {
        if let Some(_) = self.get_tree() {
            return true
        }
        return false
    }

    pub fn get_user(&mut self) -> Option<User> {
        if let Some(tree) = self.get_tree() {
            let username = tree.get("username").unwrap().clone();
            let user = self.storage.get_session(&username);

            if let Some(user) = user {
                return Some(user)
            }
            return None
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
            },
            _ => {}
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionManager<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let keystore = request.guard::<State<SessionKeystore>>().await.unwrap();
        let cookies = request.cookies();
        let storage = request.guard::<State<Storage>>().await.unwrap();
        Outcome::Success(SessionManager{ keystore, cookies, storage })
    }
}

