use aes::cipher::KeyIvInit;
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut};
use lazy_static::lazy_static;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

use crate::cfg::CONFIG;

pub struct Cipher {
    key: String,
    iv: String,
}

lazy_static! {
    pub static ref CIPHER: Cipher = Cipher::new();
}

impl Cipher {
    fn get_encryptor(&self) -> Aes128CbcEnc {
        let key = self.key.as_bytes();
        let iv = self.iv.as_bytes();
        Aes128CbcEnc::new(key.into(), iv.into())
    }

    fn get_decryptor(&self) -> Aes128CbcDec {
        let key = self.key.as_bytes();
        let iv = self.iv.as_bytes();
        Aes128CbcDec::new(key.into(), iv.into())
    }

    pub fn new() -> Cipher {
        let key = CONFIG.get::<String>("storage.key");
        let iv = CONFIG.get::<String>("storage.iv");
        Cipher { key, iv }
    }

    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        let mut buf = [0u8; 64];
        buf[..data.len()].copy_from_slice(data);
        let cipher = self.get_encryptor();
        cipher
            .encrypt_padded_mut::<Pkcs7>(&mut buf, data.len())
            .unwrap()
            .to_vec()
    }

    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        let mut buf = data.to_owned();
        let cipher = self.get_decryptor();
        cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buf)
            .unwrap()
            .to_vec()
    }
}
