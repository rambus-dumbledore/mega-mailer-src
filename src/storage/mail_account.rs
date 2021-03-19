use serde::{Serialize, Deserialize};

use crate::storage::CIPHER;

#[derive(Serialize, Deserialize)]
pub struct MailAccount {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct MailAccountEncrypted {
    pub email: String,
    pub password: Vec<u8>,
}

impl std::convert::From<MailAccount> for MailAccountEncrypted {
    fn from(account: MailAccount) -> Self {
        let enc_pwd = CIPHER.encrypt(account.password.as_bytes());
        MailAccountEncrypted{
            email: account.email,
            password: enc_pwd
        }
    }
}

impl std::convert::From<MailAccountEncrypted> for MailAccount {
    fn from(account: MailAccountEncrypted) -> Self {
        let dec_pwd = CIPHER.decrypt(account.password.as_slice());
        MailAccount{
            email: account.email,
            password: String::from_utf8(dec_pwd).unwrap()
        }
    }
}
