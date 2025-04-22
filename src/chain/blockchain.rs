use std::sync::Arc;
use tokio;

use crate::account::wallet::Wallet;
use crate::chain::block::{Block, BlockStatus};
use crate::client::network::SharedState;
use crate::client::sink::safe_send;
use crate::storage::ledger::LedgerValue;

use super::block_manager::BlockManager;

#[derive(Debug)]
pub enum BlockchainError {
    UnfinalizedBlock,
    InvalidPreviousHash,
    InvalidBlockHeight,
    InvalidProofOfWork,
}

#[allow(dead_code)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub accounts: Vec<Wallet>,
    pub current_difficulty_bits: u64,
    pub genesis_hash: Vec<u8>,
}

impl Blockchain {
    pub fn new(current_difficulty_bits: u64) -> Self {
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

    pub fn start_mining_service_async(
        blockchain: Arc<tokio::sync::Mutex<Blockchain>>,
        block_manager: Arc<tokio::sync::Mutex<BlockManager>>,
        state: Arc<SharedState>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let maybe_block = {
                    let mut bc = blockchain.lock().await;
                    let mut bm = block_manager.lock().await;
                    bm.process_block_creation(&mut bc)
                };

                if let Some(mut block) = maybe_block {
                    let height = block.height;
                    println!("Starting mining for block {}", height);

                    let bc_clone = Arc::clone(&blockchain);
                    let bm_clone = Arc::clone(&block_manager);
                    let block_clone = block.clone();
                    let mine_handle = tokio::task::spawn_blocking(move || {
                        let mut guard = tokio::sync::Mutex::blocking_lock_owned(bc_clone);
                        let mut bm = tokio::sync::Mutex::blocking_lock_owned(bm_clone);
                        block.mine(&mut *guard, &mut *bm)
                    });

                    match mine_handle.await {
                        Ok(true) => {
                            println!("Mined block {}", height);
                            let mut bm = block_manager.lock().await;
                            bm.remove_unfinalized_block(height);

                            let parsed_block = &LedgerValue::Blocks(block_clone);
                            let key = state.ledger.lock().await.get_key(&parsed_block);
                            let message = state
                                .ledger
                                .lock()
                                .await
                                .format_entry_value(&key, &parsed_block);

                            if let Err(e) = safe_send(&state.sink, &message).await {
                                eprintln!("Failed to send block: {}", e);
                            };
                        }
                        Ok(false) => eprintln!("Proof‑of‑work failed for {}", height),
                        Err(e) => eprintln!("Mining thread panicked: {}", e),
                    }
                }
            }
        })
    }
}
