use rand;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AttachRequest {
    pub code: String,
    pub username: String,
    pub expires: std::time::SystemTime,
}

impl AttachRequest {
    pub fn new(username: &String) -> AttachRequest {
        let mut rng = rand::thread_rng();
        let code = rng.gen_range(100000..999999).to_string();
        let username = username.clone();
        let expires = std::time::SystemTime::now() + std::time::Duration::from_secs(30);
        AttachRequest {
            code,
            username,
            expires,
        }
    }

    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now();
        self.expires > now
    }
}
