use lazy_static::lazy_static;
use block_modes::{BlockMode, Cbc, block_padding::Pkcs7};
use aes::Aes128;

use crate::cfg::CONFIG;

type Aes128CBC = Cbc<Aes128, Pkcs7>;

pub struct Cipher {
    key: String,
    iv: String,
}

lazy_static!{
    pub static ref CIPHER: Cipher = Cipher::new();
}

impl Cipher {
    fn get_cipher(&self) -> Aes128CBC {
        Aes128CBC::new_var(self.key.as_bytes(), self.iv.as_bytes()).unwrap()
    }

    pub fn new() -> Cipher {
        let key = CONFIG.get::<String>("storage.key");
        let iv = CONFIG.get::<String>("storage.iv");
        Cipher{ key, iv }
    }

    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        let cipher = self.get_cipher();
        cipher.encrypt_vec(data)
    }

    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        let cipher = self.get_cipher();
        cipher.decrypt_vec(data).unwrap()
    }
}
