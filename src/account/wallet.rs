use bincode::{Decode, Encode};

use crate::cryptography::signature::{get_private_key, sign};
use crate::utils::conversion::public_key_to_address;
use crate::utils::conversion::to_hex;

#[derive(Encode, Decode, Clone)]
pub struct Wallet {
    pub address: Vec<u8>,
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
}

impl Wallet {
    pub fn new(private_key: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            address: public_key_to_address(&public_key),
            private_key,
            public_key,
        }
    }

    pub fn get_address(&self) -> String {
        to_hex(&self.address)
    }

    #[allow(dead_code)] // TODO: this can be used by a contract interpreter
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        sign(
            message,
            &get_private_key(Some(&self.private_key.as_slice().try_into().unwrap())).to_bytes(),
        )
    }

    pub fn send(&self, to: &Wallet, value: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.address);
        data.extend_from_slice(&to.address);
        data.extend_from_slice(&value.to_be_bytes());
        self.sign(&data)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.address);
        data.extend_from_slice(&self.public_key);
        data
    }
}
