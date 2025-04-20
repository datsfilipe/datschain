use bincode::{Decode, Encode};

use crate::cryptography::hash::transform;
use crate::utils::conversion::to_hex;
use crate::utils::time::get_timestamp;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Transaction {
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    pub value: Vec<u64>,
    pub timestamp: u64,
    pub hash: Vec<u8>,
    pub nonce: u64,
}

impl Transaction {
    pub fn new(from: &Vec<u8>, to: &Vec<u8>, value: Vec<u64>, nonce: Option<u64>) -> Self {
        let nonce = match nonce {
            Some(nonce) => nonce,
            None => 0,
        };

        let mut s = String::new();
        s.push_str(&to_hex(&from));
        s.push_str(&to_hex(&to));
        s.push_str(&to_hex(
            &value
                .iter()
                .map(|value| value.to_be_bytes())
                .flatten()
                .collect::<Vec<u8>>(),
        ));
        s.push_str(&nonce.to_string());

        Self {
            hash: transform(&s).into_bytes(),
            from: from.clone(),
            to: to.clone(),
            value,
            nonce,
            timestamp: get_timestamp(),
        }
    }

    pub fn to_string(&self) -> String {
        to_hex(&self.hash)
    }
}
