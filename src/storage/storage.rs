use serde_cbor;
use serde::Serialize;
use serde::de::DeserializeOwned;
use redis::{FromRedisValue, ToRedisArgs, Commands};
use log::{error};

use crate::types::{Result, Error};
use crate::storage::{User, LoginRequest, AttachRequest, MailAccount};
use crate::storage::mail_account::MailAccountEncrypted;
use crate::cfg::CONFIG;

#[derive(Clone)]
pub struct Storage {
    client: redis::Client,
}

impl Storage {
    pub fn new() -> Result<Storage> {
        let client = redis::Client::open(format!("redis://{}", CONFIG.get::<String>("storage.redis")))?;
        Ok(Storage {
            client
        })
    }
}

impl Storage {
    fn get_impl<T: FromRedisValue>(&self, key: &String) -> redis::RedisResult<T> {
        let mut conn = self.client.get_connection().unwrap();
        conn.get(key.as_str())
    }

    fn set_impl<T: ToRedisArgs, KV: FromRedisValue>(&self, key: &String, value: T) -> redis::RedisResult<KV> {
        let mut conn = self.client.get_connection().unwrap();
        conn.set(key.as_str(), value)
    }

    fn del_impl<KV: FromRedisValue>(&self, key: &String) -> redis::RedisResult<KV> {
        let mut conn = self.client.get_connection().unwrap();
        conn.del(key.as_str())
    }

    fn set<T: ToRedisArgs>(&self, key: &String, value: T) -> Result<bool> {
        let res = self.set_impl(key, value);
        res.map_err(|e| {
            Error::StorageError(e)
        })
    }

    fn get<T: FromRedisValue>(&self, key: &String) -> Result<T> {
        self.get_impl::<T>(key).map_err(|e|{
            Error::from(e)
        })
    }

    fn set_bin<T: Serialize>(&self, key: &String, value: &T) -> Result<bool> {
        let data = serde_cbor::to_vec(value)?;
        self.set(key, data)
    }

    fn get_bin<T>(&self, key: &String) -> Option<T>
        where T: DeserializeOwned
    {
        let data = self.get::<Vec<u8>>(&key).ok()?;

        match serde_cbor::from_slice::<T>(data.as_slice()) {
            Ok(data) => Some(data),
            Err(e) => {
                error!("Deserialization error: {}", e);
                None
            }
        }
    }

    fn del(&self, key: &String) -> Result<()> {
        let res: redis::RedisResult<()> = self.del_impl(key);

        if res.is_ok() {
            return Ok(());
        }

        Err(Error::StorageError(res.unwrap_err()))
    }

    pub fn get_session(&self, username: &String) -> Option<User> {
        let key = format!("SESSION:{}", username);
        let data = self.get::<Vec<u8>>(&key);
        if let Ok(data) = data {
            return serde_cbor::from_slice(data.as_slice()).unwrap() // TODO
        }
        None
    }

    pub fn set_session(&self, user: &User) {
        let key = format!("SESSION:{}", user.user_name);
        let data = serde_cbor::to_vec(&user).unwrap(); // TODO
        self.set(&key, data).unwrap();
    }

    pub fn remove_session(&self, username: &String) {
        let key = format!("SESSION:{}", username);
        self.del(&key).unwrap()
    }

    pub fn get_telegram_id(&self, username: &String) -> Result<String> {
        let key = format!("TELEGRAM_ID:{}", username);
        self.get::<String>(&key)
    }

    pub fn set_telegram_id(&self, attach_request: &AttachRequest, telegram_id: &String) {
        let key = format!("TELEGRAM_ID:{}", attach_request.username);
        self.set(&key, telegram_id).unwrap();
    }

    pub fn create_login_request(&self, username: &String) -> String {
        let login_request = LoginRequest::new(username);
        let key = format!("LOGIN:{}", &login_request.code);
        self.set(&key, serde_cbor::to_vec(&login_request).unwrap()).unwrap();
        login_request.code
    }

    pub fn get_login_request(&self, code: &String) -> Option<LoginRequest> {
        let key = format!("LOGIN:{}", code);
        let data = self.get::<Vec<u8>>(&key);

        match data {
            Ok(data) => {
                match self.del(&key) {
                    Err(e) => {
                        error!("Could not delete login request by key {}: {}", code, e);
                    },
                    _ => {}
                }

                match serde_cbor::from_slice::<LoginRequest>(data.as_slice()) {
                    Ok(request) => {
                        if request.is_valid() {
                            return request.into();
                        }
                        None
                    },
                    Err(e) => {
                        error!("Deserialization error: {}", e);
                        None
                    }
                }

            },
            Err(e) => {
                error!("Could not get login request by code {}: {}", code, e);
                None
            }
        }

    }

    pub fn create_attach_request(&self, username: &String) -> Result<String> {
        let attach_request = AttachRequest::new(username);
        let key = format!("ATTACH:{}", &attach_request.code);
        self.set(&key, serde_cbor::to_vec(&attach_request)?)?;
        Ok(attach_request.code)
    }

    pub fn get_attach_request(&self, code: &String) -> Option<AttachRequest> {
        let key = format!("ATTACH:{}", code);
        let data = self.get::<Vec<u8>>(&key);
        self.del(&key).unwrap();
        match data {
            Ok(data) => {
                match serde_cbor::from_slice::<AttachRequest>(data.as_slice()) {
                    Ok(req) => {
                        if req.is_valid() {
                            return Some(req.into())
                        }
                        error!("Trying to get invalid attach request");
                        None
                    }
                    _ => None
                }
            }
            Err(e) => {
                error!("Could not get an attach request with code {}: {}", code, e);
                None
            }
        }
    }

    pub fn set_mail_account(&self, username: &String, email: &String, password: &String) -> Result<bool> {
        let key = format!("ACCOUNT:{}", username);
        let account = MailAccount{email: email.clone(), password: password.clone()};
        let encrypted_account: MailAccountEncrypted = account.into();
        self.set_bin(&key, &encrypted_account)
    }

    pub fn get_mail_account(&self, username: &String) -> Option<MailAccount> {
        let key = format!("ACCOUNT:{}", username);
        let encrypted_account = self.get_bin::<MailAccountEncrypted>(&key);
        if let Some(encrypted_account) = encrypted_account {
            let account: MailAccount = encrypted_account.into();
            return Some(account);
        }
        None
    }
}
