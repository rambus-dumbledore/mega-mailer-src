use std::sync::Arc;

use amqprs::{
    callbacks::{ChannelCallback, DefaultConnectionCallback}, channel::{
        BasicAckArguments, BasicConsumeArguments, BasicPublishArguments, Channel, ExchangeDeclareArguments, QueueBindArguments, QueueDeclareArguments
    }, connection::{Connection, OpenConnectionArguments}, Ack, BasicProperties, Cancel, CloseChannel, Nack, Return
};
use anyhow::Result;
use axum::async_trait;
use serde::{Serialize, Deserialize};
use tokio::sync::{broadcast::Sender, mpsc, Mutex};

use crate::cfg::RabbitmqCfg;

#[derive(Clone)]
pub struct Queue {
    connection: Arc<Mutex<Connection>>,
    channel: Arc<Mutex<Channel>>,
    cfg: RabbitmqCfg,
    // exchange: String,
    // routing_key: String,
    queue_name: String,
    channel_tx: Sender<()>,
    // channel_rx: Arc<Mutex<Receiver<()>>>,
}

impl Queue {
    const ROUTING_KEY: &'static str = "mailbot";
    const EXCHANGE: &'static str = "tasks";
    const EXCHANGE_TYPE: &'static str = "direct";
    
    pub async fn new(cfg: &RabbitmqCfg) -> Result<Self> {
          
        let connection = Queue::create_connection(cfg).await?;

        let channel = connection.open_channel(None).await?;
        let (channel_tx, _channel_rx) = tokio::sync::broadcast::channel(1);

        let mut queue = Self {
            connection: Arc::new(Mutex::new(connection)),
            cfg: cfg.clone(),
            channel: Arc::new(Mutex::new(channel.clone())),
            // exchange: Self::EXCHANGE.clone(),
            // routing_key: Self::ROUTING_KEY.clone(),
            queue_name: String::new(),
            channel_tx,
            // channel_rx: Arc::new(Mutex::new(channel_rx)),
        };

        queue.channel.lock().await.register_callback(queue.clone()).await?;

        let queue_params = QueueDeclareArguments::default()
            .queue(cfg.queue.clone())
            .durable(true)
            .finish();

        let (queue_name, _, _) = channel.queue_declare(queue_params).await?.unwrap();

        channel.exchange_declare(ExchangeDeclareArguments::new(&Self::EXCHANGE, Self::EXCHANGE_TYPE)).await?;
        channel.queue_bind(QueueBindArguments::new(&queue_name, &Self::EXCHANGE, &Self::ROUTING_KEY)).await?;

        queue.queue_name = queue_name;

        Ok(queue)
    }

    async fn create_connection(cfg: &RabbitmqCfg) -> Result<Connection> {
        let connection = Connection::open(&OpenConnectionArguments::new(
            &cfg.address,
            cfg.port,
            "guest",
            "guest"
        )).await?;
        connection.register_callback(DefaultConnectionCallback).await?;
        Ok(connection)
    }

    // async fn connection(&self) -> Result<Connection> {
    //     let conn = self.connection.lock().await;
    //     Ok(conn.clone())
    // }

    // pub async fn create_channel(cfg: &RabbitmqCfg, exchange: &String, routing_key: &String) -> Result<(Channel, String)> {

    // }

    pub async fn channel(&self) -> Result<(Channel, String)> {
        // let channel = self.connection().await?.open_channel(None).await?;
        // channel.register_callback(self.clone()).await?;

        // let queue_params = QueueDeclareArguments::default()
        //     .queue(self.cfg.queue.clone())
        //     .durable(true)
        //     .finish();

        // let (queue_name, _, _) = channel.queue_declare(queue_params).await?.unwrap();

        // channel.exchange_declare(ExchangeDeclareArguments::new(&Self::EXCHANGE,Self::EXCHANGE_TYPE)).await?;
        // channel.queue_bind(QueueBindArguments::new(&queue_name, &Self::EXCHANGE, Self::ROUTING_KEY)).await?;

        let channel = self.channel.lock().await.clone();
        let queue_name = self.queue_name.clone();

        Ok((channel, queue_name))
    }

    pub async fn publish(&self, msg_type: &str, msg: QueueMessage) -> Result<()> {
        let args = BasicPublishArguments::new(Self::EXCHANGE, Self::ROUTING_KEY);
        let basic_properties = BasicProperties::default()
            .with_persistence(true)
            .with_content_type("application/json")
            .with_message_type(msg_type)
            .finish();
        let content = serde_json::to_string(&msg)?;
        let (channel, _) = self.channel().await?;
        
        if let Err(e) = channel.basic_publish(basic_properties, content.into_bytes(), args).await {
            tracing::warn!("basic_publish finished with error: {e}");
        }
        Ok(())
    }

    pub async fn subscribe(&self, consumer_tag: String) -> Result<(mpsc::UnboundedReceiver<(QueueMessage, u64, u16)>, tokio::task::JoinHandle<()>)> {
        let (tx, rx) = mpsc::unbounded_channel();

        let queue = self.clone();

        let consumer_thread = tokio::spawn(async move {
            async fn worker(queue: &Queue, consumer_tag: String, tx: mpsc::UnboundedSender<(QueueMessage, u64, u16)>) -> Result<()> {
                let (channel, queue_name) = queue.channel().await?;
                let args = BasicConsumeArguments::new(&queue_name, &consumer_tag)
                    .manual_ack(true)
                    .exclusive(false)
                    .finish();
                
                let (_, mut consume_rx) = channel.basic_consume_rx(args).await?;
                let mut rx = queue.channel_tx.subscribe();
                let channel_id = channel.channel_id();

                loop {
                    tokio::select! {
                        Some(msg) = consume_rx.recv() => {
                            

                            let delivery_tag = msg.deliver
                                .map(|d| d.delivery_tag());
        
                            let task = msg.content
                                .map(|content| serde_json::from_slice::<QueueMessage>(&content))
                                .map(|msg| msg.ok())
                                .and_then(|msg| msg);
        
                            match (delivery_tag, task) {
                                (Some(delivery_tag), Some(task)) => tx.send((task, delivery_tag, channel_id))?,
                                _ => {}
                            }
                        },
                        _ = rx.recv() => {
                            break;
                        }
                    }
                }

                Ok(())
            }

            if let Err(e) = worker(&queue, consumer_tag, tx).await {
                tracing::error!("Consumer thread finished with error: {}", e);
            }
        });

        Ok((rx, consumer_thread))
    }

    pub async fn ack(&self, delivery_tag: u64, channel_id: u16) -> Result<()> {
        let (channel, _) = self.channel().await?;
        if channel_id != channel.channel_id() {
            tracing::warn!("Trying to ack delivery_tag for wrong channel id");
            return Ok(());
        }
        let ack_args = BasicAckArguments::new(delivery_tag, false);
        if let Err(e) = channel.basic_ack(ack_args).await {
            tracing::warn!("basic_ack finished with error: {e}");
        }
        Ok(())
    }
}

#[async_trait]
impl ChannelCallback for Queue {
    async fn close(&mut self, _channel: &Channel, _close: CloseChannel) -> std::result::Result<(), amqprs::error::Error> {
        if let Err(e) = self.channel_tx.send(()) {
            tracing::error!("Error in Queue ChannelCallback::close implementation: {e}");
        }

        let next_channel = self.connection.lock().await.open_channel(None).await?;
        next_channel.register_callback(self.clone()).await?;

        let queue_params = QueueDeclareArguments::default()
            .queue(self.cfg.queue.clone())
            .durable(true)
            .finish();

        let (next_queue_name, _, _) = next_channel.queue_declare(queue_params).await?.unwrap();

        next_channel.exchange_declare(ExchangeDeclareArguments::new(&Self::EXCHANGE,Self::EXCHANGE_TYPE)).await?;
        next_channel.queue_bind(QueueBindArguments::new(&next_queue_name, &Self::EXCHANGE, Self::ROUTING_KEY)).await?;

        *self.channel.lock().await = next_channel;
        self.queue_name = next_queue_name;

        Ok(())
    }
    async fn cancel(&mut self, _channel: &Channel, _cancel: Cancel) -> std::result::Result<(), amqprs::error::Error> {
        Ok(())
    }
    async fn flow(&mut self, _channel: &Channel, _active: bool) -> std::result::Result<bool, amqprs::error::Error> {
        Ok(true)
    }
    async fn publish_ack(&mut self, _channel: &Channel, _ack: Ack) {}
    async fn publish_nack(&mut self, _channel: &Channel, _nack: Nack) {}
    async fn publish_return(
        &mut self,
        _channel: &Channel,
        _ret: Return,
        _basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {}
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

#[derive(Debug, Serialize, Deserialize)]
pub enum Tasks {
    TelegramMessageTask(TelegramMessageTask)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum QueueMessage {
    Tasks(Tasks)
}