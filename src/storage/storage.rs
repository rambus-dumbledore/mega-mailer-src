use serde_cbor;
use serde::Serialize;
use serde::de::DeserializeOwned;
use redis::{FromRedisValue, ToRedisArgs, Commands};

use crate::types::{Result};
use crate::storage::{User, LoginRequest, AttachRequest, MailAccount};
use crate::storage::mail_account::MailAccountEncrypted;
use crate::cfg::CONFIG;

#[derive(Clone)]
pub struct Storage {
    client: redis::Client,
}

impl Storage {
    pub fn new() -> Storage {
        let client = redis::Client::open(format!("redis://{}", CONFIG.get::<String>("storage.redis"))).unwrap();
        Storage {
            client
        }
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

    fn set<T: ToRedisArgs>(&self, key: &String, value: T) -> Result<()> {
        let res: redis::RedisResult<()> = self.set_impl(key, value);
        if let Ok(res) = res {
            return Ok(res);
        }
        let err = res.unwrap_err();
        Err(Box::new(err))
    }

    fn get<T: FromRedisValue>(&self, key: &String) -> Option<T> {
        let res: redis::RedisResult<T> = self.get_impl(key);

        if res.is_ok() {
            return Some(res.unwrap())
        }
        println!("{}", res.err().unwrap());
        None
    }

    fn set_bin<T: Serialize>(&self, key: &String, value: &T) -> Result<()> {
        let data = serde_cbor::to_vec(value).unwrap();
        self.set(key, data)
    }

    fn get_bin<T>(&self, key: &String) -> Option<T>
        where T: DeserializeOwned
    {
        let data = self.get::<Vec<u8>>(&key);
        if let Some(data) = data {
            let item = serde_cbor::from_slice::<T>(data.as_slice());
            if let Ok(item) = item {
                return Some(item);
            }

        }
        None
    }

    fn del(&self, key: &String) -> Result<()> {
        let res: redis::RedisResult<()> = self.del_impl(key);

        if res.is_ok() {
            return Ok(());
        }

        Err(Box::new(res.unwrap_err()))
    }

    pub fn get_session(&self, username: &String) -> Option<User> {
        let key = format!("SESSION:{}", username);
        let data: Option<Vec<u8>> = self.get(&key);
        if let Some(data) = data {
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

    pub fn get_telegram_id(&self, username: &String) -> Option<String> {
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
        self.del(&key).unwrap();
        if let Some(data) = data {
            let request = serde_cbor::from_slice::<LoginRequest>(data.as_slice()).unwrap();
            if request.is_valid() {
                return request.into()
            }
        }
        None
    }

    pub fn create_attach_request(&self, username: &String) -> String {
        let attach_request = AttachRequest::new(username);
        let key = format!("ATTACH:{}", &attach_request.code);
        self.set(&key, serde_cbor::to_vec(&attach_request).unwrap()).unwrap();
        attach_request.code
    }

    pub fn get_attach_request(&self, code: &String) -> Option<AttachRequest> {
        let key = format!("ATTACH:{}", code);
        let data = self.get::<Vec<u8>>(&key);
        self.del(&key).unwrap();
        if let Some(data) = data {
            let request = serde_cbor::from_slice::<AttachRequest>(data.as_slice()).unwrap();
            if request.is_valid() {
                return request.into()
            }
        }
        None
    }

    pub fn set_mail_account(&self, username: &String, email: &String, password: &String) {
        let key = format!("ACCOUNT:{}", username);
        let account = MailAccount{email: email.clone(), password: password.clone()};
        let encrypted_account: MailAccountEncrypted = account.into();
        self.set_bin(&key, &encrypted_account).unwrap()
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
