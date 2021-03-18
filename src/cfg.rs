use config::{Config, File, Environment};
use lazy_static::lazy_static;
use log::{info, error};
use serde::Deserialize;
use std::fmt::Display;

pub struct Cfg(Config);

impl Cfg {
    pub fn get<'de, T>(&self, key: &str) -> T
    where T: Display + Deserialize<'de>
    {
        self.get_or_default(key, None)
    }

    pub fn get_or_default<'de, T>(&self, key: &str, default: Option<T>) -> T
    where T: Display + Deserialize<'de>
    {
        info!(target: "Config", "trying access to config with key \"{}\"", key);
        let res = self.0.get::<T>(key);
        if let Ok(value) = res {
            info!(target: "Config", "value \"{}\" with key \"{}\" was get successfully", value, key);
            return value;
        } else {
            let err = res.err().unwrap();
            error!(target: "Config", "error while getting value with key \"{}\"", key);
            error!(target: "Config", "{}", err);
        }
        if let Some(value) = default {
            info!(target: "Config", "using default value \"{}\" for key \"{}\"", value, key);
            return value;
        }
        panic!("x_x too baaaad");
    }
}

fn build_config() -> Cfg {
    let mut settings = Config::default();
    settings
        .merge(File::with_name("config")).unwrap()
        .merge(Environment::with_prefix("APP")).unwrap();
    Cfg(settings)
}

lazy_static!{
    pub static ref CONFIG: Cfg = build_config();
}