use rand;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    pub code: String,
    pub username: String,
    pub expires: std::time::SystemTime,
}

impl LoginRequest {
    pub fn new(username: &String) -> LoginRequest {
        let mut rng = rand::thread_rng();
        let code = rng.gen_range(100000..999999).to_string();
        let username = username.clone();
        let expires = std::time::SystemTime::now() + std::time::Duration::from_secs(30);
        LoginRequest {
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
