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
use storage::ledger::{DifficultyUpdate, Ledger};
use utils::conversion::to_hex;

fn main() {
    let mut blockchain = Blockchain::new(1);
    let mut ledger = Ledger::new();

    let (public_key1, private_key1) = generate_keypair(None);
    let wallet1 = Wallet::new(private_key1, public_key1.clone());
    blockchain.add_account(wallet1.clone());

    let (public_key2, private_key2) = generate_keypair(None);
    let wallet2 = Wallet::new(private_key2, public_key2.clone());
    blockchain.add_account(wallet2.clone());

    println!("wallet1: {}", wallet1.get_address());
    println!("wallet2: {}", wallet2.get_address());

    let initial_diff = DifficultyUpdate {
        current: blockchain.current_difficulty_bits,
        previous: 0,
        difference: blockchain.current_difficulty_bits,
    };
    let diff_key = ledger.store_difficulty(&initial_diff);
    ledger.generate_proofs("mining");

    let account1 = Wallet {
        address: wallet1.get_address().into_bytes(),
        public_key: wallet1.public_key.clone(),
        private_key: wallet1.get_hashed_pkey(),
    };

    let account2 = Wallet {
        address: wallet2.get_address().into_bytes(),
        public_key: wallet2.public_key.clone(),
        private_key: wallet2.get_hashed_pkey(),
    };

    let acc_key1 = ledger.store_account(account1);
    let acc_key2 = ledger.store_account(account2);
    ledger.generate_proofs("accounts");

    let tx = Transaction::new(&wallet1.address, &wallet2.address, vec![100], None);
    println!("tx: {}", tx.to_string());

    let mut block = Block::new(vec![tx.clone()], blockchain.genesis_hash.clone(), 0);
    println!("block before mining: {}", to_hex(&block.hash));

    block.mine(&mut blockchain);
    println!("block after mining: {}", to_hex(&block.hash));

    let genesis_hash = transform(&String::from("genesis"));
    let block_info = Block::new(vec![tx], Vec::from(genesis_hash), 1);

    let block_key = ledger.store_block(block_info);
    ledger.generate_proofs("blocks");

    let updated_diff = DifficultyUpdate {
        current: blockchain.current_difficulty_bits,
        previous: initial_diff.current,
        difference: blockchain.current_difficulty_bits - initial_diff.current,
    };
    let new_diff_key = ledger.store_difficulty(&updated_diff);
    ledger.generate_proofs("mining");

    println!("Initial difficulty: {}", ledger.format_entry(&diff_key));
    println!("Account 1: {}", ledger.format_entry(&acc_key1));
    println!("Account 2: {}", ledger.format_entry(&acc_key2));
    println!("Block: {}", ledger.format_entry(&block_key));
    println!("Updated difficulty: {}", ledger.format_entry(&new_diff_key));

    println!(
        "Verify difficulty proof: {}",
        ledger.verify_entry(&diff_key)
    );
    println!("Verify account 1 proof: {}", ledger.verify_entry(&acc_key1));
    println!("Verify account 2 proof: {}", ledger.verify_entry(&acc_key2));
    println!("Verify block proof: {}", ledger.verify_entry(&block_key));
    println!(
        "Verify updated difficulty proof: {}",
        ledger.verify_entry(&new_diff_key)
    );

    for block in blockchain.blocks.iter() {
        println!("Blockchain block: {}", to_hex(&block.hash));
    }
}
