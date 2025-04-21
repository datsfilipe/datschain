use bincode::{Decode, Encode};

use crate::chain::block_manager::BlockManager;
use crate::chain::blockchain::Blockchain;
use crate::chain::transaction::Transaction;
use crate::cryptography::hash::transform;
use crate::utils::conversion::to_hex;
use crate::utils::time::get_timestamp;

#[derive(Debug, Clone, Encode, Decode)]
pub enum BlockStatus {
    Unfinalized,
    Finalized,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Block {
    pub transactions: Vec<Transaction>,
    pub previous_hash: Vec<u8>,
    pub hash: Vec<u8>,
    pub nonce: u64,
    pub timestamp: u64,
    pub status: BlockStatus,
    pub height: u64,
}

impl Block {
    pub fn new(transactions: Vec<Transaction>, previous_hash: Vec<u8>, height: u64) -> Self {
        let block_data = Self::build_block_data(&transactions, &previous_hash, 0, get_timestamp());

        Self {
            transactions,
            previous_hash,
            hash: transform(&block_data).into_bytes(),
            nonce: 0,
            timestamp: get_timestamp(),
            status: BlockStatus::Unfinalized,
            height,
        }
    }

    fn build_block_data(
        transactions: &Vec<Transaction>,
        previous_hash: &Vec<u8>,
        nonce: u64,
        timestamp: u64,
    ) -> String {
        let mut block_data = String::new();

        block_data.push_str(&to_hex(previous_hash));

        let transactions_str = transactions
            .iter()
            .map(|tx| tx.to_string())
            .collect::<String>();

        let merkle_root = transform(&transactions_str);
        block_data.push_str(&merkle_root);
        block_data.push_str(&nonce.to_string());
        block_data.push_str(&timestamp.to_string());

        block_data
    }

    fn meets_difficulty(&self, hash: &Vec<u8>, target_bits: u64) -> bool {
        let mut leading_zeros = 0;

        for byte in hash {
            if *byte == 0 {
                leading_zeros += 8;
                continue;
            }

            let mut mask = 0x80;
            while mask > 0 && (*byte & mask) == 0 {
                leading_zeros += 1;
                mask >>= 1;
            }
            break;
        }

        leading_zeros >= target_bits
    }

    fn get_difficulty_target(&self, chain: &Blockchain) -> u64 {
        const INITIAL_DIFFICULTY_BITS: u64 = 16;
        const TARGET_BLOCK_TIME: u64 = 600; // 10 minutes in seconds
        const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016; // ~2 weeks of blocks

        if self.height == 0 {
            return INITIAL_DIFFICULTY_BITS;
        }

        if self.height % DIFFICULTY_ADJUSTMENT_INTERVAL != 0 {
            return chain.current_difficulty_bits;
        }

        let adjustment_start_height = self.height - DIFFICULTY_ADJUSTMENT_INTERVAL;
        let adjustment_start_block = chain
            .get_block_by_height(adjustment_start_height)
            .expect("Failed to get adjustment start block");

        let time_diff = self.timestamp - adjustment_start_block.timestamp;
        let expected_time = TARGET_BLOCK_TIME * DIFFICULTY_ADJUSTMENT_INTERVAL;
        let mut new_difficulty = chain.current_difficulty_bits;

        if time_diff < expected_time / 4 {
            new_difficulty += 1;
        } else if time_diff > expected_time * 4 {
            new_difficulty = new_difficulty.saturating_sub(1);
        } else {
            let adjustment = (expected_time as f64) / (time_diff as f64);
            let adjustment = adjustment.max(0.25).min(4.0);
            if adjustment > 1.0 {
                new_difficulty += 1;
            } else if adjustment < 1.0 {
                new_difficulty = new_difficulty.saturating_sub(1);
            }
        }

        new_difficulty
    }

    pub fn mine(&mut self, blockchain: &mut Blockchain, block_manager: &mut BlockManager) -> bool {
        println!("Mining block {}", self.height);
        let target_bits = self.get_difficulty_target(blockchain);
        let max_attempts = 1_000_000;

        for _ in 0..max_attempts {
            self.nonce += 1;
            let block_data = Self::build_block_data(
                &self.transactions,
                &self.previous_hash,
                self.nonce,
                self.timestamp,
            );

            let hash = transform(&block_data).into_bytes();
            if self.meets_difficulty(&hash, target_bits) {
                self.hash = hash;
                self.finalize(self.clone(), blockchain, block_manager);
                return true;
            }
        }

        false
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> bool {
        if matches!(self.status, BlockStatus::Finalized) {
            return false;
        }

        self.transactions.push(transaction);
        true
    }

    fn finalize(
        &mut self,
        copy: Block,
        blockchain: &mut Blockchain,
        blocK_manager: &mut BlockManager,
    ) {
        self.status = BlockStatus::Finalized;
        match blockchain.add_block(copy) {
            Ok(_) => blocK_manager.remove_unfinalized_block(self.height),
            Err(e) => println!("{:?}", e),
        }
    }

    pub fn verify(&self, target_bits: u64) -> bool {
        let block_data = Self::build_block_data(
            &self.transactions,
            &self.previous_hash,
            self.nonce,
            self.timestamp,
        );

        let hash = transform(&block_data).into_bytes();
        if hash != self.hash {
            return false;
        }

        self.meets_difficulty(&hash, target_bits)
    }
}
