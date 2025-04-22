use crate::{
    account::wallet::Wallet,
    chain::block::Block,
    cryptography::hash::transform,
    storage::{level_db::Storage, tree::Tree},
    utils::conversion::{hash_to_32bit_array, to_hex},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub key: [u8; 32],
    pub value: LedgerValue,
    pub proof: Option<LedgerProof>,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct LedgerProof {
    pub tree_identifier: String,
    pub proof_indices: Vec<usize>,
    pub proof_data: Vec<u8>,
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

pub struct Ledger {
    pub mining_tree: Tree,
    pub accounts_tree: Tree,
    pub blocks_tree: Tree,
    pub entries: HashMap<[u8; 32], LedgerEntry>,
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
        let version = self.entries.get(&key).map_or(0, |e| e.version + 1);
        let entry = LedgerEntry {
            key,
            value,
            proof: Some(proof),
            version,
        };
        println!(
            "Saving entry key: {}, version: {}",
            to_hex(&key),
            entry.version
        );
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
        let (ok, tree_proof_bytes, tree_indices) = {
            let tree = match tree_identifier {
                "mining" => &mut self.mining_tree,
                "accounts" => &mut self.accounts_tree,
                "blocks" => &mut self.blocks_tree,
                _ => {
                    eprintln!("Unknown tree identifier: {}", tree_identifier);
                    return None;
                }
            };

            tree.insert(key);
            tree.commit()
        };

        if ok {
            println!(
                "Commit successful for tree '{}', key {}",
                tree_identifier,
                to_hex(&key)
            );
            let proof = LedgerProof {
                tree_identifier: tree_identifier.to_string(),
                proof_indices: tree_indices,
                proof_data: tree_proof_bytes,
            };
            let entry = self.save_entry(key, entry_value, proof.clone());
            let formatted_value = self.format_entry_value(&entry.key, &entry.value);
            match storage.store(&key, formatted_value).await {
                Ok(_) => {
                    println!("Successfully stored entry {} in LevelDB", to_hex(&key));
                    Some(proof)
                }
                Err(e) => {
                    eprintln!("Failed to store entry {} in LevelDB after commit: {}. Rolling back tree state.", to_hex(&key), e);
                    match tree_identifier {
                        "mining" => self.mining_tree.rollback(),
                        "accounts" => self.accounts_tree.rollback(),
                        "blocks" => self.blocks_tree.rollback(),
                        _ => {}
                    };
                    None
                }
            }
        } else {
            eprintln!(
                "Merkle tree commit failed for tree '{}', key {}. Tree automatically rolled back.",
                tree_identifier,
                to_hex(&key)
            );
            None
        }
    }

    pub async fn commit_peer_state(
        &mut self,
        key: [u8; 32],
        entry_value: LedgerValue,
        tree_identifier: &str,
        storage: &mut Storage,
    ) -> Option<()> {
        self.commit_with_identifier(key, entry_value, tree_identifier, storage)
            .await
            .map(|_| ())
    }

    pub async fn sync_client_block(
        &mut self,
        key: [u8; 32],
        entry_value: LedgerValue,
        tree_identifier: &str,
        storage: &mut Storage,
    ) -> Option<()> {
        self.commit_peer_state(key, entry_value, tree_identifier, storage)
            .await
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

    pub fn format_entry_value(&mut self, key: &[u8; 32], value: &LedgerValue) -> String {
        let value_str = match serde_json::to_string(&value) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "Failed to serialize LedgerValue for key {}: {}",
                    to_hex(key),
                    e
                );
                format!("\"Error serializing value: {}\"", e)
            }
        };

        format!(
            "{{\"key\":\"{}\", \"value\":{}, \"version\":{}}}",
            to_hex(key),
            value_str,
            0,
        )
    }

    pub fn get_latest_block_key(&self) -> Option<[u8; 32]> {
        self.blocks_tree.get_leaves().last().copied()
    }
    pub fn get_latest_account_key(&self) -> Option<[u8; 32]> {
        self.accounts_tree.get_leaves().last().copied()
    }
    pub fn get_latest_mining_key(&self) -> Option<[u8; 32]> {
        self.mining_tree.get_leaves().last().copied()
    }
}
