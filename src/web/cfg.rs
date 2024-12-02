use common::cfg::*;
use config::Config;

#[derive(Clone)]
pub struct WebServerCfg {
    pub web: WebCfg,
    pub storage: StorageCfg,
    pub bot: BotCfg,
}

impl TryFrom<Config> for WebServerCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: Config) -> std::result::Result<Self, Self::Error> {
        let web = WebCfg::try_from(&cfg)?;
        let storage = StorageCfg::try_from(&cfg)?;
        let bot = BotCfg::try_from(&cfg)?;
        Ok(WebServerCfg { web, storage, bot })
    }
}
