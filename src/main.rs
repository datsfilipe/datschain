mod account;
mod chain;
mod cryptography;
mod storage;
mod utils;

use account::wallet::Wallet;
use chain::block::Block;
use chain::blockchain::Blockchain;
use chain::transaction::Transaction;
use cryptography::hash::transform;
use cryptography::signature::generate_keypair;
use storage::tree::Tree;
use utils::conversion::{hash_to_32bit_array, to_hex};

fn main() {
    let mut blockchain = Blockchain::new(1);
    let mut diffictulty_tree = Tree::new("difficulty".to_string());
    diffictulty_tree.insert(hash_to_32bit_array(transform(&String::from("0"))));
    diffictulty_tree.commit();

    let (public_key1, private_key1) = generate_keypair(None);
    let wallet1 = Wallet::new(private_key1, public_key1);

    let (public_key2, private_key2) = generate_keypair(None);
    let wallet2 = Wallet::new(private_key2, public_key2);

    println!("wallet1: {}", wallet1.get_address());
    println!("wallet2: {}", wallet2.get_address());

    let message = b"hello world";
    let signed_message = wallet1.sign(message);
    println!("signed_message: {}", to_hex(&signed_message));

    let tx = Transaction::new(&wallet1.address, &wallet2.address, vec![100]);
    println!("tx: {}", tx.to_string());

    let mut block = Block::new(vec![tx], blockchain.genesis_hash.clone(), 0);
    println!("block: {}", to_hex(&block.hash));

    block.mine(&mut blockchain);
    println!("block: {}", to_hex(&block.hash));

    diffictulty_tree.insert(hash_to_32bit_array(transform(&String::from(format!(
        "{}",
        blockchain.current_difficulty_bits
    )))));
    diffictulty_tree.commit();

    let leaves = diffictulty_tree.get_leaves();
    let proof = diffictulty_tree.proof(leaves);
    println!("proof: {}", proof);

    for block in blockchain.blocks.iter() {
        println!("block: {}", to_hex(&block.hash));
    }
}
