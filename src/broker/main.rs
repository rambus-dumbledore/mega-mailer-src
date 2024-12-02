mod cfg;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use async_zmq::SinkExt;
use cfg::BrokerSvcCfg;
use common::{
    cfg::build_config,
    ctrlc_handler::set_ctrlc_handler,
    queues::{BrokerMessage, BrokerRequest},
};
use tokio_stream::StreamExt;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};

#[derive(Clone)]
pub struct Broker {
    cfg: BrokerSvcCfg,
}

impl Broker {
    pub fn new(cfg: BrokerSvcCfg) -> Result<Self> {
        Ok(Self { cfg })
    }

    async fn rep_thread(
        &self,
        run: Arc<AtomicBool>,
        tx: tokio::sync::mpsc::Sender<BrokerMessage>,
    ) -> Result<()> {
        let mut rep_sock =
            async_zmq::reply(&format!("tcp://*:{}", self.cfg.broker.rep_port))?.bind()?;

        loop {
            if run.load(Ordering::Relaxed) == false {
                break;
            }

            tokio::select! {
                Some(Ok(messages)) = rep_sock.next() => {
                    for msg in messages {
                        let data: Vec<u8> = msg.as_str().map(|str| str.as_bytes().to_vec()).ok_or(anyhow::anyhow!(""))?;
                        let r: BrokerRequest = serde_json::from_slice(&data)?;

                        match r.payload {
                            common::queues::BrokerRequestPayload::Tasks(task) => {
                                let m = BrokerMessage {
                                    message_id: r.id,
                                    payload: common::queues::BrokerMessagePayload::Tasks(task)
                                };
                                let _ = tx.send(m).await;
                            },
                            common::queues::BrokerRequestPayload::Ack(_ack) => { continue; },
                        }
                    }

                    rep_sock.send("").await?;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
        }
        Ok(())
    }

    async fn pub_thread(
        &self,
        run: Arc<AtomicBool>,
        mut rx: tokio::sync::mpsc::Receiver<BrokerMessage>,
    ) -> Result<()> {
        let mut pub_sock =
            async_zmq::publish(&format!("tcp://*:{}", self.cfg.broker.pub_port))?.bind()?;

        loop {
            if run.load(Ordering::Relaxed) == false {
                break;
            }

            tokio::select! {
                Some(msg) = rx.recv() => {
                    tracing::debug!("pub_thread received message: {:?}", msg);
                    let topic: Vec<u8> = "tasks".as_bytes().to_vec();
                    let data = serde_json::to_vec(&msg)?;
                    pub_sock.send(vec![topic, data].into()).await?;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {},
            }
        }
        Ok(())
    }
}

async fn main_impl() -> Result<()> {
    let r = Arc::new(AtomicBool::new(true));
    set_ctrlc_handler(r.clone())?;

    let cfg = build_config::<BrokerSvcCfg>()?;
    let broker = Broker::new(cfg)?;
    let (tx, rx) = tokio::sync::mpsc::channel::<BrokerMessage>(1);

    let b = broker.clone();
    let r2 = r.clone();
    tokio::spawn(async move {
        if let Err(e) = b.pub_thread(r2, rx).await {
            tracing::warn!("broker::pub_thread finished with error: {}", e);
        }
    });

    tracing::info!("Broker started");

    broker.rep_thread(r.clone(), tx).await?;

    Ok(())
}

fn main() {
    let _guard = common::sentry::init_sentry();

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_filter(LevelFilter::INFO);

    Registry::default()
        .with(sentry::integrations::tracing::layer())
        .with(fmt_layer)
        .try_init()
        .unwrap();

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(main_impl()) {
        tracing::error!("Broker finished with error: {}", e)
    };
}
