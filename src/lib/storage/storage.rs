use bb8_redis::redis::AsyncCommands;
use anyhow::Result;

use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use crate::cfg::Cfg;
use crate::sessions::WebAppUser;
use crate::storage::mail_account::MailAccountEncrypted;
use crate::storage::MailAccount;
use crate::types::TelegramMessageTask;

use super::cipher::Cipher;

type PGPool = bb8::Pool<bb8_postgres::PostgresConnectionManager<bb8_postgres::tokio_postgres::NoTls>>;
type RedisPool = bb8::Pool<bb8_redis::RedisConnectionManager>;

#[derive(Clone)]
pub struct Storage {
    pg: PGPool,
    redis: RedisPool,
}

impl Storage {
    pub async fn new(cfg: &Cfg) -> Result<Self> {
        let pg_config = bb8_postgres::tokio_postgres::Config::from_str(&cfg.storage.postgres)?;
        let pg_man = bb8_postgres::PostgresConnectionManager::new(pg_config, bb8_postgres::tokio_postgres::NoTls);
        let redis_man = bb8_redis::RedisConnectionManager::new(cfg.storage.redis.clone())?;
        let pg =  bb8::Pool::builder().build(pg_man).await?;
        let redis =  bb8::Pool::builder().build(redis_man).await?;
        Ok(Self{
            pg,
            redis
        })
    }

    pub async fn get_session_v2(&self, cookie: &str) -> Result<Option<WebAppUser>> {
        let key = format!("SESSION:{}", cookie);
        let mut conn = self.redis.get().await?;
        let id: Option<i64> = conn.get(key).await?;
        match id {
            Some(id) => Ok(Some(id.into())),
            None => Ok(None)
        }
    }

    pub async fn remove_session_v2(&self, cookie: &str) -> Result<bool> {
        let key = format!("SESSION:{}", cookie);
        let mut conn = self.redis.get().await?;
        let res = conn.del(key).await?;
        Ok(res)
    }

    pub async fn set_session_v2(&self, cookie: &str, user: &WebAppUser) -> Result<bool> {
        let key = format!("SESSION:{}", cookie);
        let mut conn = self.redis.get().await?;
        let res = conn.set(&key, user.id).await?;
        conn.expire(&key, 3600).await?;
        Ok(res)
    }

    pub async fn set_mail_account(
        &self,
        user: &WebAppUser,
        email: &String,
        password: &String,
        cipher: &Cipher
    ) -> Result<()> {
        let account = MailAccount {
            email: email.clone(),
            password: password.clone(),
        };
        let encrypted_account = account.encrypt(cipher);
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            INSERT INTO "mail_accounts" ("id", "username", "password")
            VALUES ($3, $1, $2)
            ON CONFLICT ("id") DO UPDATE SET "username" = $1, "password" = $2;
        "#).await?;
        conn.execute(&statement, &[&encrypted_account.email, &encrypted_account.password, &user.id]).await?;
        Ok(())
    }

    pub async fn get_mail_account(&self, user: &WebAppUser, cipher: &Cipher) -> Result<Option<MailAccount>> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            SELECT "username", "password"
            FROM "mail_accounts"
            WHERE "id" = $1
        "#).await?;
        let rows = conn.query(&statement, &[&user.id]).await?;
        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        let enc_account = MailAccountEncrypted{
            email: row.get(0),
            password: row.get(1)
        };
        Ok(Some(enc_account.decrypt(cipher)))
    }

    pub async fn add_processed_mails(&self, user: &WebAppUser, uids: &[u32]) -> Result<()> {
        let key = format!("PROCESSED_MAIL:{}", user.id);
        let mut conn = self.redis.get().await?;
        for uid in uids {
            conn.sadd(&key, *uid).await?;
        }
        Ok(())
    }

    pub async fn filter_unprocessed(&self, user: &WebAppUser, uids: &[&u32]) -> Result<Vec<u32>> {
        let mut unprocessed = Vec::<u32>::new();
        let key = format!("PROCESSED_MAIL:{}", user.id);
        let mut conn = self.redis.get().await?;

        for uid in uids {
            let exists: bool = conn.sismember(&key, **uid).await?;
            if !exists {
                unprocessed.push(**uid);
            }
        }

        Ok(unprocessed)
    }

    pub async fn is_checking_enabled(&self, user: &WebAppUser) -> Result<bool> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            SELECT "checking" FROM "users"
            WHERE "id" = $1
        "#).await?;
        let row = conn.query_one(&statement, &[&user.id]).await?;
        Ok(row.get(0))
    }

    pub async fn disable_checking(&self, user: &WebAppUser) -> Result<()> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            UPDATE "users"
            SET "checking" = false
            WHERE "id" = $1
        "#).await?;
        conn.execute(&statement, &[&user.id]).await?;
        Ok(())
    }

    pub async fn enable_checking(&self, user: &WebAppUser) -> Result<()> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            UPDATE "users"
            SET "checking" = true
            WHERE "id" = $1
        "#).await?;
        conn.execute(&statement, &[&user.id]).await?;
        Ok(())
    }

    pub async fn get_users_for_checking(&self) -> Result<HashSet<WebAppUser>> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            SELECT "id" FROM "users"
            WHERE "checking" = true
        "#).await?;
        let users = conn.query(&statement, &[]).await?.into_iter().
            map(|row| row.get::<usize, i64>(0).into())
            .collect();
        Ok(users)
    }

    pub async fn get_send_message_tasks_queue(&self) -> Result<BTreeMap<String, TelegramMessageTask>> {
        let key = format!("TELEGRAM_MESSAGE_QUEUE");
        let mut conn = self.redis.get().await?;
        let tasks_queue = conn.hgetall(key).await?;
        Ok(tasks_queue)
    }

    pub async fn add_send_message_task_to_queue(&self, task: TelegramMessageTask) -> Result<bool> {
        let key = format!("TELEGRAM_MESSAGE_QUEUE");
        let field = uuid::Uuid::new_v4().to_string();
        let mut conn = self.redis.get().await?;
        let res = conn.hset(&key, &field, task).await?;
        Ok(res)
    }

    pub async fn remove_send_message_task_from_queue(&self, id: &String) -> Result<bool> {
        let key = format!("TELEGRAM_MESSAGE_QUEUE");
        let mut conn = self.redis.get().await?;
        let res = conn.hdel(&key, id).await?;
        Ok(res)
    }

    pub async fn get_user_working_hours(&self, user: &WebAppUser) -> Result<[u8; 2]> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
             SELECT "start", "end"
             FROM "working_hours"
             WHERE "id" = $1
        "#).await?;
        let row = conn.query_one(&statement, &[&user.id]).await?;
        let begin: i32 = row.get(0);
        let end: i32 = row.get(1);
        Ok([begin as u8, end as u8])
    }

    pub async fn set_user_working_hours(&self, user: &WebAppUser, wh: &[u8; 2]) -> Result<()> {
       let conn = self.pg.get().await?;
       let statement = conn.prepare(r#"
            UPDATE "working_hours"
            SET "start" = $1, "end" = $2
            WHERE "id" = $3
       "#).await?;
       conn.execute(&statement, &[&(wh[0] as i32), &(wh[1] as i32), &user.id]).await?;
       Ok(())
    }

    pub async fn get_important_emails(&self, user: &WebAppUser) -> Result<Vec<String>> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            SELECT unnest("important_emails") from "users"
            WHERE "id" = $1
        "#).await?;
        let rows = conn.query(&statement, &[&user.id]).await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn set_important_emails(&self, user: &WebAppUser, emails: &Vec<String>) -> Result<bool> {
        let conn = self.pg.get().await?;
        let emails_array = postgres_array::Array::from_vec(emails.clone(), 0);
        let statement = conn.prepare(r#"
            UPDATE "users"
            SET "important_emails" = $1
            WHERE "id" = $2
        "#).await?;
        conn.execute(&statement, &[&emails_array, &user.id]).await?;
        Ok(true)
    }

    pub async fn get_important_tags(&self, user: &WebAppUser) -> Result<Vec<String>> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            SELECT unnest("important_tags") from "users"
            WHERE "id" = $1
        "#).await?;
        let rows = conn.query(&statement, &[&user.id]).await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn set_important_tags(&self, user: &WebAppUser, tags: &Vec<String>) -> Result<()> {
        let tags_array = postgres_array::Array::from_vec(tags.clone(), 0);
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            UPDATE "users"
            SET "important_tags" = $1
            WHERE "id" = $2
        "#).await?;
        conn.execute(&statement, &[&tags_array, &user.id]).await?;
        Ok(())
    }

    pub async  fn set_heartbeat(&self, service: &String, timestamp: i64) -> Result<bool> {
        let mut conn = self.redis.get().await?;
        let res = conn.hset(&String::from("HEARTBEAT"), service, timestamp).await?;
        Ok(res)
    }

    pub async fn get_heartbeat(&self) -> Result<BTreeMap<String, i64>> {
        let mut conn = self.redis.get().await?;
        let res = conn.hgetall(&String::from("HEARTBEAT")).await?;
        Ok(res)
    }

    pub async fn migrate_pg(&self, sql: &str) -> Result<()> {
        let conn = self.pg.get().await?;
        conn.simple_query(sql).await?;
        Ok(())
    }

    pub async fn register_user(&self, user: &WebAppUser) -> Result<()> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            INSERT INTO "users" ("id")
            VALUES ($1);
        "#).await?;
        conn.execute(&statement, &[&user.id]).await?;
        let statement = conn.prepare(r#"
            INSERT INTO "working_hours" ("id", "start", "end")
            VALUES ($1, 10, 19);
        "#).await?;
        conn.execute(&statement, &[&user.id]).await?;
        Ok(())
    }

    pub async fn is_user_registed(&self, user: &WebAppUser) -> Result<bool> {
        let conn = self.pg.get().await?;
        let statement = conn.prepare(r#"
            SELECT 1 FROM "users"
            WHERE "id" = $1
        "#).await?;
        let row = conn.query_opt(&statement, &[&user.id]).await?;
        Ok(row.is_some())
    }
}
