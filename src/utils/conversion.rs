pub fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

pub fn public_key_to_address(public_key: &Vec<u8>) -> Vec<u8> {
    let mut address = vec![0u8; 20];
    address.copy_from_slice(&public_key[0..20]);
    address
}

pub fn hash_to_32bit_array(hash: String) -> [u8; 32] {
    let bytes = hash.as_bytes();
    let mut array = [0u8; 32];
    let len = bytes.len().min(32);
    array[..len].copy_from_slice(&bytes[..len]);
    array
}
