use crate::utils::conversion::public_key_to_address;

pub struct Wallet {
    pub address: Vec<u8>,
    public_key: Vec<u8>,
}

impl Wallet {
    pub fn new(public_key: Vec<u8>) -> Self {
        Self {
            address: public_key_to_address(&public_key),
            public_key,
        }
    }
}
