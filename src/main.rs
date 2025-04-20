mod account;
mod chain;
mod cryptography;
mod utils;

use account::wallet::Wallet;
use chain::block::Block;
use chain::blockchain::Blockchain;
use chain::transaction::Transaction;
use cryptography::signature::generate_keypair;
use utils::conversion::to_hex;

fn main() {
    let mut blockchain = chain::blockchain::Blockchain::new(1);

    let (public_key1, private_key1) = generate_keypair(None);
    let wallet1 = Wallet::new(private_key1, public_key1);

    let (public_key2, private_key2) = generate_keypair(None);
    let wallet2 = Wallet::new(private_key2, public_key2);

    println!("wallet1: {}", to_hex(&wallet1.address));
    println!("wallet2: {}", to_hex(&wallet2.address));

    let message = b"hello world";
    let signed_message = wallet1.sign(message);
    println!("signed_message: {}", to_hex(&signed_message));

    let tx = Transaction::new(&wallet1.address, &wallet2.address, vec![100]);
    println!("tx: {}", tx.to_string());

    let mut block = Block::new(vec![tx], blockchain.genesis_hash.clone(), 0);
    println!("block: {}", to_hex(&block.hash));

    block.mine(&mut blockchain);
    println!("block: {}", to_hex(&block.hash));

    for block in blockchain.blocks.iter() {
        println!("block: {}", to_hex(&block.hash));
    }
}
