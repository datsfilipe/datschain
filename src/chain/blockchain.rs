use crate::account::wallet::Wallet;
use crate::chain::block::{Block, BlockStatus};

#[derive(Debug)]
pub enum BlockchainError {
    UnfinalizedBlock,
    InvalidPreviousHash,
    InvalidBlockHeight,
    InvalidProofOfWork,
}

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub accounts: Vec<Wallet>,
    pub current_difficulty_bits: u32,
    pub genesis_hash: Vec<u8>,
}

impl Blockchain {
    pub fn new(current_difficulty_bits: u32) -> Self {
        let genesis_block = Block::new(vec![], vec![], 0);

        Self {
            genesis_hash: genesis_block.hash.clone(),
            blocks: vec![genesis_block],
            accounts: vec![],
            current_difficulty_bits,
        }
    }

    pub fn add_block(&mut self, block: Block) -> Result<(), BlockchainError> {
        if matches!(block.status, BlockStatus::Unfinalized) {
            return Err(BlockchainError::UnfinalizedBlock);
        }

        let expected_prev_hash = match self.blocks.last() {
            Some(last_block) => &last_block.hash,
            None => &self.genesis_hash,
        };

        if block.previous_hash != *expected_prev_hash {
            return Err(BlockchainError::InvalidPreviousHash);
        }

        if block.height != self.blocks.len() as u64 {
            return Err(BlockchainError::InvalidBlockHeight);
        }

        if !block.verify(self.current_difficulty_bits) {
            return Err(BlockchainError::InvalidProofOfWork);
        }

        self.blocks.push(block);
        Ok(())
    }

    pub fn add_account(&mut self, account: Wallet) {
        self.accounts.push(account);
    }

    pub fn get_block_by_height(&self, height: u64) -> Option<&Block> {
        self.blocks.get(height as usize)
    }
}
