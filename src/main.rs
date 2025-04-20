mod cryptography;

use cryptography::hash::{transform, verify};

fn main() {
    let message = String::from("hello world");
    let data = transform(&message);
    println!("{:?}", "0x".to_owned() + &data);

    let verify = verify(&message, data);
    println!("{:?}", verify);
}
