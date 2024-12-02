use anyhow::{anyhow, Result};
use config::{Config, Environment, File};
use std::path::PathBuf;

#[derive(Clone)]
pub struct WebCfg {
    pub address: std::net::SocketAddr,
    pub static_path: std::path::PathBuf,
    pub cookie_key: String,
}

impl TryFrom<&Config> for WebCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: &Config) -> std::result::Result<Self, Self::Error> {
        let address: std::net::SocketAddr = cfg.get_string("web.address")?.parse()?;
        let static_path: PathBuf = cfg.get_string("web.static_path")?.into();
        if !static_path.exists() {
            return Err(anyhow!(
                "`web.static_path` value is not correct: {}",
                static_path.display()
            ));
        }
        let cookie_key = cfg.get_string("web.cookie_key")?;
        Ok(WebCfg {
            address,
            static_path,
            cookie_key,
        })
    }
}

#[derive(Clone)]
pub struct StorageCfg {
    pub redis: String,
    pub postgres: String,
    pub key: String,
    pub iv: String,
}

impl TryFrom<&Config> for StorageCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: &Config) -> std::result::Result<Self, Self::Error> {
        let redis = cfg.get_string("storage.redis")?;
        let postgres = cfg.get_string("storage.postgres")?;
        let key = cfg.get_string("storage.key")?;
        let iv = cfg.get_string("storage.iv")?;
        Ok(StorageCfg {
            redis,
            postgres,
            key,
            iv,
        })
    }
}

#[derive(Clone)]
pub struct BotCfg {
    pub token: String,
}

impl TryFrom<&Config> for BotCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: &Config) -> std::result::Result<Self, Self::Error> {
        let token = cfg.get_string("bot.secret")?;
        Ok(BotCfg { token })
    }
}

#[derive(Clone)]
pub struct MailCfg {
    pub address: String,
    pub port: u16,
}

impl TryFrom<&Config> for MailCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: &Config) -> std::result::Result<Self, Self::Error> {
        let address = cfg.get_string("mail.address")?;
        let port: u16 = cfg.get_int("mail.port")? as u16;
        Ok(MailCfg { address, port })
    }
}

#[derive(Clone)]
pub struct BrokerCfg {
    pub address: String,
    pub pub_port: u16,
    pub rep_port: u16,
}

impl TryFrom<&Config> for BrokerCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: &Config) -> std::result::Result<Self, Self::Error> {
        let address = cfg.get_string("broker.address")?;
        let pub_port = cfg.get_int("broker.pub_port")? as u16;
        let rep_port = cfg.get_int("broker.rep_port")? as u16;
        Ok(BrokerCfg {
            address,
            pub_port,
            rep_port,
        })
    }
}

pub fn build_config<T: TryFrom<Config, Error = anyhow::Error>>() -> Result<T> {
    let cfg = Config::builder()
        .add_source(File::with_name("config"))
        .add_source(Environment::with_prefix("APP"))
        .build()
        .unwrap();
    cfg.try_into()
}
