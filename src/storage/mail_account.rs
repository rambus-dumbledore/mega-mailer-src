use serde::{Serialize, Deserialize};
use block_modes::{BlockMode, Cbc, block_padding::Pkcs7};
use aes::Aes128;
use hex_literal::hex;

type Aes128CBC = Cbc<Aes128, Pkcs7>;

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
        let key = hex!("000102030405060708090a0b0c0d0e0f");
        let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
        let cipher = Aes128CBC::new_var(&key, &iv).unwrap();

        let enc_pwd = cipher.encrypt_vec(account.password.as_bytes());
        MailAccountEncrypted{
            email: account.email,
            password: enc_pwd
        }
    }
}

impl std::convert::From<MailAccountEncrypted> for MailAccount {
    fn from(account: MailAccountEncrypted) -> Self {
        let key = hex!("000102030405060708090a0b0c0d0e0f");
        let iv = hex!("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
        let cipher = Aes128CBC::new_var(&key, &iv).unwrap();

        let dec_pwd = cipher.decrypt_vec(account.password.as_slice()).unwrap();
        MailAccount{
            email: account.email,
            password: String::from_utf8(dec_pwd).unwrap()
        }
    }
}
