use bincode::{decode_from_slice, encode_into_slice, Decode, Encode};
use std::collections::HashMap;

use crate::account::wallet::Wallet;
use crate::chain::block::Block;
use crate::cryptography::hash::transform;
use crate::storage::{level_db::Storage, tree::Tree};
use crate::utils::conversion::{hash_to_32bit_array, to_hex};

#[derive(Encode, Decode)]
pub enum DecodedData {
    Mining(DifficultyUpdate),
    Accounts(Wallet),
    Blocks(Block),
}

pub struct Ledger {
    pub mining_tree: Tree,
    pub accounts_tree: Tree,
    pub blocks_tree: Tree,
    pub entries: HashMap<[u8; 32], LedgerEntry>,
}

#[allow(dead_code)]
pub struct LedgerEntry {
    pub key: [u8; 32],
    pub value: Vec<u8>,
    pub proof: Option<LedgerProof>,
    pub version: u64,
}

pub struct LedgerProof {
    tree_identifier: String,
    proof_indices: Vec<usize>,
    proof_data: Vec<u8>,
}

#[derive(Encode, Decode)]
pub struct DifficultyUpdate {
    pub current: u64,
    pub previous: u64,
    pub difference: u64,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            mining_tree: Tree::new("mining".to_string()),
            accounts_tree: Tree::new("accounts".to_string()),
            blocks_tree: Tree::new("blocks".to_string()),
            entries: HashMap::new(),
        }
    }

    pub fn get_key(&self, value: &DecodedData) -> [u8; 32] {
        let encoded = self.encode_value(value);
        hash_to_32bit_array(transform(&format!("{:?}", encoded)))
    }

    pub async fn commit_with_identifier(
        &mut self,
        key: [u8; 32],
        tree_identifier: &str,
        storage: &mut Storage,
    ) -> Option<LedgerProof> {
        let mut proof: Option<Vec<u8>> = None;
        let mut proof_indices: Vec<usize> = vec![];
        match tree_identifier {
            "mining" => {
                self.mining_tree.insert(key);
                let (ok, tree_proof, tree_indices) = self.mining_tree.commit();
                if ok {
                    proof = Some(tree_proof);
                    proof_indices = tree_indices;
                }
            }
            "accounts" => {
                self.accounts_tree.insert(key);
                let (ok, tree_proof, tree_indices) = self.accounts_tree.commit();
                if ok {
                    proof = Some(tree_proof);
                    proof_indices = tree_indices;
                }
            }
            "blocks" => {
                self.blocks_tree.insert(key);
                let (ok, tree_proof, tree_indices) = self.blocks_tree.commit();
                if ok {
                    proof = Some(tree_proof);
                    proof_indices = tree_indices;
                }
            }
            _ => {}
        }

        if let Some(proof) = proof {
            storage
                .store(&key, self.format_entry(&key))
                .await
                .expect("Failed to store proof");

            return Some(LedgerProof {
                tree_identifier: tree_identifier.to_string(),
                proof_indices,
                proof_data: proof,
            });
        }

        None
    }

    pub async fn sync_client_block(
        &mut self,
        key: [u8; 32],
        entry: Vec<u8>,
        tree_identifier: &str,
        storage: &mut Storage,
    ) -> Option<bool> {
        let proof = self
            .commit_with_identifier(key, tree_identifier, storage)
            .await;

        if let Some(proof) = proof {
            let entry = LedgerEntry {
                key,
                proof: Some(proof),
                value: entry,
                version: 0,
            };

            self.entries.insert(key, entry);

            Some(true)
        } else {
            None
        }
    }

    pub fn verify_entry(&self, key: &[u8; 32]) -> bool {
        if let Some(entry) = self.entries.get(key) {
            if let Some(proof) = &entry.proof {
                let tree = match proof.tree_identifier.as_str() {
                    "mining" => &self.mining_tree,
                    "accounts" => &self.accounts_tree,
                    "blocks" => &self.blocks_tree,
                    _ => return false,
                };

                return tree.verify_proof_bytes(&[*key], &proof.proof_indices, &proof.proof_data);
            }
        }
        false
    }

    pub fn format_entry(&self, key: &[u8; 32]) -> String {
        if let Some(entry) = self.entries.get(key) {
            let key_hex = to_hex(key);
            let value_type = self.determine_value_type(&entry.value);

            format!(
                "key={}, value={}, version={}",
                key_hex, value_type, entry.version
            )
        } else {
            "Entry not found".to_string()
        }
    }

    pub fn decode_value<T>(&self, value: &[u8]) -> Result<T, bincode::error::DecodeError>
    where
        T: Decode<()>,
    {
        decode_from_slice::<T, _>(value, bincode::config::standard()).map(|(value, _)| value)
    }

    pub fn encode_value<T>(&self, value: &T) -> Vec<u8>
    where
        T: Encode,
    {
        let mut serialized = vec![0u8; 256];
        encode_into_slice(value, &mut serialized, bincode::config::standard()).unwrap();
        serialized
    }

    fn determine_value_type(&self, value: &[u8]) -> String {
        if let Ok(difficulty) = self.decode_value::<DifficultyUpdate>(value) {
            format!(
                "{{current:{}, previous:{}, difference:{}}}",
                difficulty.current, difficulty.previous, difficulty.difference
            )
        } else if let Ok(account) = self.decode_value::<Wallet>(value) {
            format!("{{address:{:?}, deleted:{}}}", account.address, false)
        } else if let Ok(block) = self.decode_value::<Block>(value) {
            format!(
                "{{hash:{:?}, height:{}, transactions:{:?}}}",
                block.hash, block.height, block.transactions
            )
        } else {
            format!("{{bytes:{:?}}}", value)
        }
    }

    pub fn get_latest_block_key(&self) -> Option<[u8; 32]> {
        let leaves = self.blocks_tree.get_leaves();
        leaves.last().copied()
    }
}
