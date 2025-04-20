pub struct Wallet {
    address: Vec<u8>,
    public_key: Vec<u8>,
}

fn public_key_to_address(public_key: &Vec<u8>) -> Vec<u8> {
    let mut address = vec![0u8; 20];
    address.copy_from_slice(&public_key[0..20]);
    address
}

impl Wallet {
    pub fn new(public_key: Vec<u8>) -> Self {
        Self {
            address: public_key_to_address(&public_key),
            public_key,
        }
    }
}
