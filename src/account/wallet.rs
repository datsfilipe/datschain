pub struct Wallet {
    address: Vec<u8>,
    public_key: Vec<u8>,
}

impl Wallet {
    pub fn new(address: Vec<u8>, public_key: Vec<u8>) -> Self {
        Self {
            address,
            public_key,
        }
    }
}
