use std::fmt::Write;

pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

pub fn to_string(bytes: &[u8]) -> String {
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
