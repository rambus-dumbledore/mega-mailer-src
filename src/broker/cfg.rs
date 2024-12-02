use common::cfg::{BrokerCfg, StorageCfg};
use config::Config;

#[derive(Clone)]
pub struct BrokerSvcCfg {
    pub broker: BrokerCfg,
    pub storage: StorageCfg,
}

impl TryFrom<Config> for BrokerSvcCfg {
    type Error = anyhow::Error;

    fn try_from(value: Config) -> Result<Self, Self::Error> {
        let broker = BrokerCfg::try_from(&value)?;
        let storage = StorageCfg::try_from(&value)?;
        Ok(Self { broker, storage })
    }
}
