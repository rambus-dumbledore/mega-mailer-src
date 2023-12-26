use serde::{Deserialize, Serialize};

use super::cipher::Cipher;
#[derive(Serialize, Deserialize, Debug)]
pub struct MailAccount {
    pub email: String,
    pub password: String,
}


#[derive(Serialize, Deserialize)]
pub struct MailAccountEncrypted {
    pub email: String,
    pub password: Vec<u8>,
}

impl MailAccount {
    pub fn encrypt(self, cipher: &Cipher) -> MailAccountEncrypted {
        let password = cipher.encrypt(self.password.as_bytes());
        MailAccountEncrypted { email: self.email, password }
    }
}

impl MailAccountEncrypted {
    pub fn decrypt(self, cipher: &Cipher) -> MailAccount {
        let password = cipher.decrypt(self.password.as_slice());
        MailAccount {
            email: self.email,
            password: String::from_utf8(password).unwrap(),
        }
    }
}
