use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::account::wallet::Wallet;
use crate::chain::block::Block;
use crate::cryptography::hash::transform;
use crate::storage::{level_db::Storage, tree::Tree};
use crate::utils::conversion::{hash_to_32bit_array, to_hex};
use crate::utils::encoding::decode_from_base64;

pub struct Ledger {
    pub mining_tree: Tree,
    pub accounts_tree: Tree,
    pub blocks_tree: Tree,
    pub entries: HashMap<[u8; 32], LedgerEntry>,
}

#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub key: [u8; 32],
    pub value: LedgerValue,
    pub proof: Option<LedgerProof>,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct LedgerProof {
    tree_identifier: String,
    proof_indices: Vec<usize>,
    proof_data: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DifficultyUpdate {
    pub current: u64,
    pub previous: u64,
    pub difference: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum LedgerValue {
    Mining(DifficultyUpdate),
    Accounts(Wallet),
    Blocks(Block),
}

#[derive(Debug, Deserialize)]
pub struct DeserializedLedgerValue {
    pub value: LedgerValue,
    pub version: u64,
    pub key: String,
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

    pub fn get_key(&self, value: &LedgerValue) -> [u8; 32] {
        hash_to_32bit_array(transform(&format!("{:?}", value)))
    }

    pub fn save_entry(
        &mut self,
        key: [u8; 32],
        value: LedgerValue,
        proof: LedgerProof,
    ) -> LedgerEntry {
        let entry = LedgerEntry {
            key,
            value,
            proof: Some(proof),
            version: 0,
        };

        self.entries.insert(key, entry.clone());
        entry
    }

    pub async fn commit_with_identifier(
        &mut self,
        key: [u8; 32],
        entry_value: LedgerValue,
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
            let proof = Some(LedgerProof {
                tree_identifier: tree_identifier.to_string(),
                proof_indices,
                proof_data: proof,
            });

            let entry = self.save_entry(key, entry_value, proof.clone().unwrap());
            storage
                .store(&key, self.format_entry_value(&entry.key, &entry.value))
                .await
                .expect("Failed to store proof");

            return proof;
        }

        None
    }

    pub async fn sync_client_block(
        &mut self,
        key: [u8; 32],
        entry_value: LedgerValue,
        tree_identifier: &str,
        storage: &mut Storage,
    ) -> Option<bool> {
        let proof = self
            .commit_with_identifier(key, entry_value, tree_identifier, storage)
            .await;
        match proof {
            Some(_) => Some(true),
            None => None,
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

    pub fn format_entry_value(&self, key: &[u8; 32], value: &LedgerValue) -> String {
        if let Some(entry) = self.entries.get(key) {
            match serde_json::to_string(&value) {
                Ok(value) => format!(
                    "key={}, value={}, version={}",
                    to_hex(&entry.key),
                    value,
                    entry.version
                ),
                Err(e) => format!(
                    "key={}, value={}, version={}, error={}",
                    to_hex(&entry.key),
                    serde_json::to_string(&value)
                        .unwrap_or("Failed to serialize value".to_string()),
                    entry.version,
                    e
                ),
            }
        } else {
            "Entry not found".to_string() // TODO: return error
        }
    }

    pub fn get_latest_block_key(&self) -> Option<[u8; 32]> {
        let leaves = self.blocks_tree.get_leaves();
        leaves.last().copied()
    }

    pub fn get_latest_account_key(&self) -> Option<[u8; 32]> {
        let leaves = self.accounts_tree.get_leaves();
        leaves.last().copied()
    }

    pub fn get_latest_mining_key(&self) -> Option<[u8; 32]> {
        let leaves = self.mining_tree.get_leaves();
        leaves.last().copied()
    }
}
