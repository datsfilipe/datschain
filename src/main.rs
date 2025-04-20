mod cryptography;
mod utils;

use cryptography::hash::{transform, verify};
use cryptography::signature;
use utils::conversion::to_hex;

fn main() {
    let message = String::from("hello world");
    let data = transform(&message);
    println!("{:?}", "0x".to_owned() + &data);

    let verify = verify(&message, data);
    println!("{:?}", verify);

    let (secret_key, public_key) = signature::generate_keypair();
    println!("{:?}", to_hex(&secret_key));
    println!("{:?}", to_hex(&public_key));

    let bytes = message.into_bytes();
    let signature = signature::sign(&bytes, &secret_key.try_into().unwrap());
    println!("{:?}", to_hex(&signature));

    let verify = signature::verify(
        &bytes,
        &signature.try_into().unwrap(),
        &public_key.try_into().unwrap(),
    );
    println!("{:?}", verify);
}
