use std::time::{Duration, Instant};

use hashlink::LinkedHashMap;

use crate::chain::{
    block::{Block, BlockStatus},
    blockchain::Blockchain,
    transaction::Transaction,
};

#[derive(Debug)]
pub struct BlockManager {
    pending_transactions: Vec<Transaction>,
    last_block_time: Instant,
    block_interval: Duration,
    unfinalized_blocks: LinkedHashMap<u64, Block>,
}

impl BlockManager {
    pub fn new(block_interval_secs: u64) -> Self {
        Self {
            pending_transactions: Vec::new(),
            last_block_time: Instant::now(),
            block_interval: Duration::from_secs(block_interval_secs),
            unfinalized_blocks: LinkedHashMap::new(),
        }
    }

    pub fn add_transaction(&mut self, blockchain: &mut Blockchain, transaction: Transaction) {
        self.pending_transactions.push(transaction.clone());

        let last_block = self.unfinalized_blocks.values_mut().last();
        match last_block {
            Some(last_block) => {
                last_block.add_transaction(transaction);
            }
            None => {
                let mut last_block = blockchain.blocks.last().unwrap().clone();
                if matches!(last_block.status, BlockStatus::Finalized) {
                    let new_block = self.process_block_creation(blockchain);
                    match new_block {
                        Some(mut new_block) => {
                            new_block.add_transaction(transaction);
                        }
                        None => {}
                    }
                } else {
                    last_block.add_transaction(transaction);
                }
            }
        }
    }

    pub fn process_block_creation(&mut self, blockchain: &mut Blockchain) -> Option<Block> {
        if self.pending_transactions.is_empty() {
            return None;
        }

        if Instant::now().duration_since(self.last_block_time) < self.block_interval {
            return None;
        }

        let height = blockchain.blocks.len() as u64;
        let previous_hash = match blockchain.blocks.last() {
            Some(last_block) => last_block.hash.clone(),
            None => blockchain.genesis_hash.clone(),
        };

        let transactions = std::mem::take(&mut self.pending_transactions);
        let new_block = Block::new(transactions, previous_hash, height);
        let block_copy = new_block.clone();
        self.unfinalized_blocks.insert(height, block_copy);

        self.last_block_time = Instant::now();

        Some(new_block)
    }

    pub fn get_unfinalized_block(&self, height: u64) -> Option<&Block> {
        self.unfinalized_blocks.get(&height)
    }

    pub fn remove_unfinalized_block(&mut self, height: u64) {
        self.unfinalized_blocks.remove(&height);
    }
}
