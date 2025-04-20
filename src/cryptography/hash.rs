use keccak_asm::Digest;
use keccak_asm::Keccak256;

pub fn transform(input: &String) -> String {
    let mut keccak = Keccak256::new();
    keccak.update(input.as_bytes());
    let result = keccak.finalize();
    let hex_hash = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    hex_hash
}

pub fn verify(input: &String, hash: String) -> bool {
    let mut keccak = Keccak256::new();
    keccak.update(input.as_bytes());
    let result = keccak.finalize();
    let hex_hash = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    hex_hash == hash
}
