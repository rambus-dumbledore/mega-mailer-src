use common::cfg::*;
use config::Config;


#[derive(Clone)]
pub struct MailCheckerCfg {
    pub storage: StorageCfg,
    pub bot: BotCfg,
    pub mail: MailCfg,
}

impl TryFrom<Config> for MailCheckerCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: Config) -> std::result::Result<Self, Self::Error> {
        let storage = StorageCfg::try_from(&cfg)?;
        let bot = BotCfg::try_from(&cfg)?;        
        let mail = MailCfg::try_from(&cfg)?;
        Ok(MailCheckerCfg{ storage, bot, mail })
    }
}