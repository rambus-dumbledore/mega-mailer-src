use common::cfg::*;
use config::Config;


#[derive(Clone)]
pub struct TelegramBotCfg {
    pub storage: StorageCfg,
    pub bot: BotCfg,
    pub mail: MailCfg,
    pub rabbit: RabbitmqCfg,
}

impl TryFrom<Config> for TelegramBotCfg {
    type Error = anyhow::Error;

    fn try_from(cfg: Config) -> std::result::Result<Self, Self::Error> {
        let storage = StorageCfg::try_from(&cfg)?;
        let bot = BotCfg::try_from(&cfg)?;        
        let mail = MailCfg::try_from(&cfg)?;
        let rabbit = RabbitmqCfg::try_from(&cfg)?;
        Ok(TelegramBotCfg{ storage, bot, mail, rabbit })
    }
}