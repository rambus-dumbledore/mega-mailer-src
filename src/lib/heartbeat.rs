use crate::storage::Storage;
use tracing::error;
use std::pin::Pin;
use std::sync::Arc;

pub struct HeartbeatService {
    key: String,
    storage: Pin<Arc<Storage>>,
}

impl HeartbeatService {
    pub fn new(key: String, storage: Pin<Arc<Storage>>) -> Self {
        HeartbeatService { key, storage }
    }

    pub fn run(&self) {
        let (key, storage) = (self.key.clone(), self.storage.clone());
        tokio::spawn( async move {
            loop {
                let res = storage.set_heartbeat(&key, chrono::Utc::now().timestamp()).await;
                if let Err(e) = res {
                    error!(target: "Heartbeat", "Could not access to database: {}", e);
                }
    
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            }
        });
    }
}
