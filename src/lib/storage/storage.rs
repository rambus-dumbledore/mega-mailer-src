use log::error;
use redis::{Commands, FromRedisValue, ToRedisArgs};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_cbor;
use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

use crate::cfg::CONFIG;
use crate::storage::mail_account::MailAccountEncrypted;
use crate::storage::{AttachRequest, LoginRequest, MailAccount, User};
use crate::types::{Error, Result, TelegramMessageTask};

pub trait BaseStorage {}

#[derive(Clone)]
pub struct RedisStorage<T: BaseStorage> {
    client: redis::Client,
    connection: Arc<RwLock<redis::Connection>>,
    phantom: PhantomData<T>,
}

impl<T: BaseStorage> RedisStorage<T> {
    pub fn new() -> Result<RedisStorage<T>> {
        let client =
            redis::Client::open(format!("redis://{}", CONFIG.get::<String>("storage.redis")))?;
        let connection = Arc::new(RwLock::new(client.get_connection()?));
        Ok(RedisStorage {
            client,
            connection,
            phantom: PhantomData,
        })
    }
}

impl<Type: BaseStorage> RedisStorage<Type> {
    fn get_impl<T: FromRedisValue>(&self, key: &String) -> Result<T> {
        self.connection
            .write()?
            .get(key.as_str())
            .map_err(|e| Error::from(e))
    }

    fn set_impl<T: ToRedisArgs, KV: FromRedisValue>(&self, key: &String, value: T) -> Result<KV> {
        self.connection
            .write()?
            .set(key.as_str(), value)
            .map_err(|e| Error::from(e))
    }

    fn del_impl<KV: FromRedisValue>(&self, key: &String) -> Result<KV> {
        self.connection
            .write()?
            .del(key.as_str())
            .map_err(|e| Error::from(e))
    }

    fn sadd_impl<T: ToRedisArgs, KV: FromRedisValue>(&self, key: &String, value: T) -> Result<KV> {
        self.connection
            .write()?
            .sadd(key.as_str(), value)
            .map_err(|e| Error::from(e))
    }

    fn sismember_impl<T: ToRedisArgs, KV: FromRedisValue>(
        &self,
        key: &String,
        value: T,
    ) -> Result<KV> {
        self.connection
            .write()?
            .sismember(key.as_str(), value)
            .map_err(|e| Error::from(e))
    }

    fn srem_impl<T: ToRedisArgs, KV: FromRedisValue>(&self, key: &String, value: T) -> Result<KV> {
        self.connection
            .write()?
            .srem(key.as_str(), value)
            .map_err(|e| Error::from(e))
    }

    fn smembers_impl<KV: FromRedisValue>(&self, key: &String) -> Result<KV> {
        self.connection
            .write()?
            .smembers(key.as_str())
            .map_err(|e| Error::from(e))
    }

    fn hkeys_impl<KV: FromRedisValue>(&self, key: &String) -> Result<KV> {
        self.connection
            .write()?
            .hkeys(key.as_str())
            .map_err(|e| Error::from(e))
    }

    fn hgetall_impl<KV: FromRedisValue>(&self, key: &String) -> Result<KV> {
        self.connection
            .write()?
            .hgetall(key.as_str())
            .map_err(|e| Error::from(e))
    }

    fn hget_impl<KV: FromRedisValue>(&self, key: &String, field: &String) -> Result<KV> {
        self.connection
            .write()?
            .hget(key.as_str(), field.as_str())
            .map_err(|e| Error::from(e))
    }

    fn hset_impl<KV: ToRedisArgs, R: FromRedisValue>(
        &self,
        key: &String,
        field: &String,
        value: KV,
    ) -> Result<R> {
        self.connection
            .write()?
            .hset(key.as_str(), field.as_str(), value)
            .map_err(|e| Error::from(e))
    }

    fn hdel_impl<R: FromRedisValue>(&self, key: &String, field: &String) -> Result<R> {
        self.connection
            .write()?
            .hdel(key.as_str(), field.as_str())
            .map_err(|e| Error::from(e))
    }

    fn set<T: ToRedisArgs>(&self, key: &String, value: T) -> Result<bool> {
        let res = self.set_impl(key, value);
        res.map_err(|e| Error::from(e))
    }

    fn get<T: FromRedisValue>(&self, key: &String) -> Result<T> {
        self.get_impl::<T>(key).map_err(|e| Error::from(e))
    }

    fn set_bin<T: Serialize>(&self, key: &String, value: &T) -> Result<bool> {
        let data = serde_cbor::to_vec(value)?;
        self.set(key, data)
    }

    fn get_bin<T>(&self, key: &String) -> Option<T>
    where
        T: DeserializeOwned,
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
        let res = self.del_impl::<()>(key);

        if res.is_ok() {
            return Ok(());
        }

        Err(Error::from(res.unwrap_err()))
    }

    fn sadd<T: ToRedisArgs>(&self, key: &String, value: T) -> Result<bool> {
        let res = self.sadd_impl::<T, u8>(key, value);
        if let Ok(res) = res {
            return Ok(res == 1);
        }

        Err(Error::from(res.unwrap_err()))
    }

    fn sismember<T: ToRedisArgs>(&self, key: &String, value: T) -> Result<bool> {
        let res = self.sismember_impl::<T, u8>(key, value);
        if let Ok(res) = res {
            return Ok(res == 1);
        }

        Err(Error::from(res.unwrap_err()))
    }

    fn srem<T: ToRedisArgs>(&self, key: &String, value: T) -> Result<bool> {
        let res = self.srem_impl::<T, u8>(key, value);
        if let Ok(res) = res {
            return Ok(res == 1);
        }

        Err(Error::from(res.unwrap_err()))
    }

    fn smembers<KV: FromRedisValue>(&self, key: &String) -> Result<KV> {
        let res = self.smembers_impl::<KV>(key);
        if let Ok(res) = res {
            return Ok(res);
        }

        Err(Error::from(res.err().unwrap()))
    }

    fn hkeys<KV: FromRedisValue>(&self, key: &String) -> Result<KV> {
        self.hkeys_impl::<KV>(key)
    }

    fn hgetall<KV: FromRedisValue>(&self, key: &String) -> Result<KV> {
        self.hgetall_impl::<KV>(key)
    }

    fn hget<KV: FromRedisValue>(&self, key: &String, field: &String) -> Result<KV> {
        self.hget_impl::<KV>(key, field)
    }

    fn hset<KV: ToRedisArgs + FromRedisValue, R: FromRedisValue>(
        &self,
        key: &String,
        field: &String,
        value: KV,
    ) -> Result<R> {
        self.hset_impl::<KV, R>(key, field, value)
    }

    fn hdel<R: FromRedisValue>(&self, key: &String, field: &String) -> Result<R> {
        self.hdel_impl(key, field)
    }
}

#[derive(Clone)]
pub struct MainStorage;

impl BaseStorage for MainStorage {}

impl RedisStorage<MainStorage> {
    pub fn get_session(&self, username: &String) -> Option<User> {
        let key = format!("SESSION:{}", username);
        let data = self.get::<Vec<u8>>(&key);
        if let Ok(data) = data {
            return serde_cbor::from_slice(data.as_slice()).unwrap_or_else(|e| {
                error!("{}", Error::from(e));
                None
            });
        }
        None
    }

    pub fn set_session(&self, user: &User) -> Result<bool> {
        let key = format!("SESSION:{}", user.username);
        let data = serde_cbor::to_vec(&user)?;
        self.set(&key, data)
    }

    pub fn remove_session(&self, username: &String) {
        let key = format!("SESSION:{}", username);
        self.del(&key).unwrap()
    }

    pub fn get_telegram_id(&self, username: &String) -> Result<String> {
        let key = format!("TELEGRAM_ID:{}", username);
        self.get::<String>(&key)
    }

    pub fn get_username(&self, telegram_id: &String) -> Result<String> {
        let key = format!("USERNAME:{}", telegram_id);
        self.get(&key)
    }

    pub fn set_telegram_id(&self, attach_request: &AttachRequest, telegram_id: &String) {
        let key = format!("TELEGRAM_ID:{}", attach_request.username);
        self.set(&key, telegram_id).unwrap();

        let key = format!("USERNAME:{}", telegram_id);
        self.set(&key, &attach_request.username).unwrap();
    }

    pub fn create_login_request(&self, username: &String) -> String {
        let login_request = LoginRequest::new(username);
        let key = format!("LOGIN:{}", &login_request.code);
        self.set(&key, serde_cbor::to_vec(&login_request).unwrap())
            .unwrap();
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
                    }
                    _ => {}
                }

                match serde_cbor::from_slice::<LoginRequest>(data.as_slice()) {
                    Ok(request) => {
                        if request.is_valid() {
                            return request.into();
                        }
                        None
                    }
                    Err(e) => {
                        error!("Deserialization error: {}", e);
                        None
                    }
                }
            }
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
            Ok(data) => match serde_cbor::from_slice::<AttachRequest>(data.as_slice()) {
                Ok(req) => {
                    if req.is_valid() {
                        return Some(req.into());
                    }
                    error!("Trying to get invalid attach request");
                    None
                }
                _ => None,
            },
            Err(e) => {
                error!("Could not get an attach request with code {}: {}", code, e);
                None
            }
        }
    }

    pub fn set_mail_account(
        &self,
        username: &String,
        email: &String,
        password: &String,
    ) -> Result<bool> {
        let key = format!("ACCOUNT:{}", username);
        let account = MailAccount {
            email: email.clone(),
            password: password.clone(),
        };
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

    pub fn add_processed_mails(&self, username: &String, uids: &[u32]) -> Result<()> {
        let key = format!("PROCESSED_MAIL:{}", username);
        for uid in uids {
            let res = self.sadd(&key, *uid);
            if res.is_err() {
                return Err(res.unwrap_err());
            }
        }
        Ok(())
    }

    pub fn filter_unprocessed(&self, username: &String, uids: &[&u32]) -> Result<Vec<u32>> {
        let mut unprocessed = Vec::<u32>::new();
        let key = format!("PROCESSED_MAIL:{}", username);

        for uid in uids {
            let res = self.sismember(&key, **uid);
            if res.is_err() {
                return Err(res.unwrap_err());
            }
            if !res.unwrap() {
                unprocessed.push(*uid.clone());
            }
        }

        Ok(unprocessed)
    }

    pub fn is_checking_enabled(&self, username: &String) -> Result<bool> {
        let key = format!("CHECKING_ENABLED");
        self.sismember(&key, username)
    }

    pub fn disable_checking(&self, username: &String) -> Result<bool> {
        let key = format!("CHECKING_ENABLED");
        self.srem(&key, username)
    }

    pub fn enable_checking(&self, username: &String) -> Result<bool> {
        let key = format!("CHECKING_ENABLED");
        self.sadd(&key, username)
    }

    pub fn get_usernames_for_checking(&self) -> Result<HashSet<String>> {
        let key = format!("CHECKING_ENABLED");
        self.smembers(&key)
    }

    pub fn set_user_avatar(&self, username: &String, filename: &String) -> Result<bool> {
        let key = format!("AVATAR:{}", username);
        self.set(&key, filename)
    }

    pub fn get_user_avatar(&self, username: &String) -> Result<String> {
        let key = format!("AVATAR:{}", username);
        self.get::<String>(&key)
    }

    pub fn get_send_message_tasks_queue(&self) -> Result<BTreeMap<String, TelegramMessageTask>> {
        let key = format!("TELEGRAM_MESSAGE_QUEUE");
        self.hgetall(&key)
    }

    pub fn add_send_message_task_to_queue(&self, task: TelegramMessageTask) -> Result<bool> {
        let key = format!("TELEGRAM_MESSAGE_QUEUE");
        let field = uuid::Uuid::new_v4().to_string();
        self.hset(&key, &field, task)
    }

    pub fn remove_send_message_task_from_queue(&self, id: &String) -> Result<bool> {
        let key = format!("TELEGRAM_MESSAGE_QUEUE");
        self.hdel(&key, id)
    }

    pub fn get_user_working_hours(&self, username: &String) -> Option<Vec<u8>> {
        let key = format!("WORKING_HOURS:{}", username);
        self.get_bin(&key)
    }
    
    pub fn set_user_working_hours(&self, username: &String, wh: &Vec<u8>) -> Result<bool> {
        let key = format!("WORKING_HOURS:{}", username);
        self.set_bin(&key, wh)
    }

    pub fn get_important_emails(&self, username: &String) -> Option<Vec<String>> {
        let key = format!("IMPORTANT_EMAILS:{}", username);
        self.smembers(&key).ok()
    }

    pub fn add_important_email(&self, username: &String, email: &String) -> Result<bool> {
        let key = format!("IMPORTANT_EMAILS:{}", username);
        self.sadd(&key, email)
    }
    
    pub fn remove_important_email(&self, username: &String, email: &String) -> Result<bool> {
        let key = format!("IMPORTANT_EMAILS:{}", username);
        self.srem(&key, email)
    }
}

pub type Storage = RedisStorage<MainStorage>;
