use std::path::PathBuf;
use anyhow::{anyhow, Result};
use config::{Config, Environment, File};

#[derive(Clone)]
pub struct WebCfg {
    pub address: std::net::SocketAddr,
    pub static_path: std::path::PathBuf,
    pub cookie_key: String,
}

#[derive(Clone)]
pub struct StorageCfg {
    pub redis: String,
    pub postgres: String,
    pub key: String,
    pub iv: String,
}

#[derive(Clone)]
pub struct BotCfg {
    pub token: String,
}

#[derive(Clone)]
pub struct MailCfg {
    pub address: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct Cfg {
    pub debug: bool,
    pub web: WebCfg,
    pub storage: StorageCfg,
    pub bot: BotCfg,
    pub mail: MailCfg,
}

impl Cfg {
    pub fn build(cfg: Config) -> Result<Self> {
        let debug = cfg.get_bool("debug")?;

        let address: std::net::SocketAddr = cfg.get_string("web.address")?.parse()?;
        let static_path: PathBuf = cfg.get_string("web.static_path")?.into();
        if !static_path.exists() {
            return Err(anyhow!("`web.static_path` value is not correct: {}", static_path.display()));
        }
        let cookie_key = cfg.get_string("web.cookie_key")?;
        let web = WebCfg{ address, static_path, cookie_key };

        let redis = cfg.get_string("storage.redis")?;
        let postgres = cfg.get_string("storage.postgres")?;
        let key = cfg.get_string("storage.key")?;
        let iv = cfg.get_string("storage.iv")?;
        let storage = StorageCfg{ redis, postgres, key, iv };

        let token = cfg.get_string("bot.secret")?;
        let bot = BotCfg{ token };

        let address = cfg.get_string("mail.address")?;
        let port: u16 = cfg.get_int("mail.port")? as u16;
        let mail = MailCfg{ address, port };

        Ok(Cfg{ debug, web, storage, bot, mail })
    }
}

pub fn build_config() -> Result<Cfg> {
    let cfg = Config::builder()
        .add_source(File::with_name("config"))
        .add_source(Environment::with_prefix("APP"))
        .build()
        .unwrap();
    Cfg::build(cfg)
}
