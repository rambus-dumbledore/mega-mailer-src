use std::sync::Arc;
use crate::storage::Storage;
use log::error;

pub struct HeartbeatService {
    key: String,
    storage: Arc<Storage>,
}

impl HeartbeatService {
    pub fn new(key: String, storage: Arc<Storage>) -> Self {
        HeartbeatService{
            key, storage
        }
    }

    pub fn run(&self) {
        let (key, storage) = (self.key.clone(), self.storage.clone());
        std::thread::spawn(move || {
            loop {
                let res = storage.set_heartbeat(&key, chrono::Utc::now().timestamp());
                if let Err(e) = res {
                    error!(target: "Heartbeat", "Could not access to database: {}", e);
                }

                std::thread::sleep(std::time::Duration::from_secs(15));
            }
        });
    }
}
