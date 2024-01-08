use common::cfg::*;
use config::Config;

#[derive(Clone)]
pub struct WebServerCfg {
    pub debug: bool,
    pub web: WebCfg,
    pub storage: StorageCfg,
    pub bot: BotCfg,
    pub mail: MailCfg,
}

impl TryFrom<Config> for WebServerCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: Config) -> std::result::Result<Self, Self::Error> {
        let debug = cfg.get_bool("debug")?;
        let web = WebCfg::try_from(&cfg)?;
        let storage = StorageCfg::try_from(&cfg)?;
        let bot = BotCfg::try_from(&cfg)?;        
        let mail = MailCfg::try_from(&cfg)?;
        Ok(WebServerCfg{ debug, web, storage, bot, mail })
    }
}
