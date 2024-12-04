use anyhow::Result;
use async_zmq::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver, Sender},
};
use uuid::Uuid;

use crate::cfg::BrokerCfg;
use crate::retry;

#[derive(Clone)]
pub struct BrokerClient {
    cfg: BrokerCfg,
}

use teloxide_core::types::UserId;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelegramMessageTask {
    pub to: UserId,
    pub text: String,
    pub send_after: chrono::DateTime<chrono::Utc>,
    pub important: bool,
}

impl TelegramMessageTask {
    pub fn can_send_now(&self) -> bool {
        let now = chrono::Utc::now();
        if now > self.send_after {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tasks {
    TelegramMessageTask(TelegramMessageTask),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ack {
    pub message_id: uuid::Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BrokerMessagePayload {
    Tasks(Tasks),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerMessage {
    pub message_id: uuid::Uuid,
    pub payload: BrokerMessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrokerRequestPayload {
    Tasks(Tasks),
    Ack(Ack),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrokerRequest {
    pub id: Uuid,
    pub payload: BrokerRequestPayload,
}

impl BrokerClient {
    pub fn new(cfg: BrokerCfg) -> Result<Self> {
        Ok(Self { cfg })
    }

    pub async fn send(&self, payload: BrokerRequestPayload) -> Result<()> {
        let id = uuid::Uuid::new_v4();
        let m = crate::queues::BrokerRequest { id, payload };
        let data = serde_json::to_vec(&m)?;

        tracing::info!("send {}", String::from_utf8_lossy(&data));

        async fn send_impl(data: Vec<u8>, cfg: BrokerCfg) -> Result<()> {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                let local = tokio::task::LocalSet::new();
                let handle = local.spawn_local(async move {
                    let requestor =
                        async_zmq::request(&format!("tcp://{}:{}", cfg.address, cfg.rep_port))?
                            .connect()?;
                    requestor.send(data).await?;
                    let _ = requestor.recv().await?;

                    Ok::<(), anyhow::Error>(())
                });
                local.run_until(handle).await?
            })
        }

        retry! { send_impl(data.clone(), self.cfg.clone()).await }?;

        Ok(())
    }

    pub async fn subscribe(&self) -> Result<Receiver<BrokerMessage>> {
        let sub =
            async_zmq::subscribe(&format!("tcp://{}:{}", self.cfg.address, self.cfg.pub_port))?
                .connect()?;
        sub.set_subscribe(&"tasks")?;

        let (tx, rx) = channel(10);

        async fn sub_impl(mut sub: async_zmq::Subscribe, tx: Sender<BrokerMessage>) -> Result<()> {
            while let Some(messages) = sub.next().await {
                match messages {
                    Ok(msgs) => {
                        for msg in msgs {
                            tracing::info!("received {:?}", msg.as_str());

                            if msg.get_more() {
                                continue;
                            }

                            let str = msg.as_str().ok_or(anyhow::anyhow!(""))?;
                            let m: BrokerMessage = serde_json::from_str(&str).unwrap();
                            tx.send(m).await.ok();
                        }
                    }
                    Err(_) => {}
                }
            }

            Ok(())
        }
        spawn(sub_impl(sub, tx));
        Ok(rx)
    }

    pub async fn ack(&self, message_id: uuid::Uuid) -> Result<()> {
        let payload = BrokerRequestPayload::Ack(Ack { message_id });
        self.send(payload).await?;
        Ok(())
    }
}
