use bincode::{config, decode_from_slice, encode_into_slice, Decode, Encode};
use std::collections::HashMap;

use crate::account::wallet::Wallet;
use crate::chain::block::Block;
use crate::cryptography::hash::transform;
use crate::storage::tree::Tree;
use crate::utils::conversion::{hash_to_32bit_array, to_hex};

pub struct Ledger {
    pub mining_tree: Tree,
    pub accounts_tree: Tree,
    pub blocks_tree: Tree,
    pub entries: HashMap<[u8; 32], LedgerEntry>,
}

#[allow(dead_code)]
pub struct LedgerEntry {
    key: [u8; 32],
    value: Vec<u8>,
    proof: Option<LedgerProof>,
    version: u64,
}

struct LedgerProof {
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

    pub fn store_difficulty(&mut self, diff_update: &DifficultyUpdate) -> [u8; 32] {
        let mut serialized = vec![0u8; 34];
        encode_into_slice(&diff_update, &mut serialized, config::standard()).unwrap();
        let key = transform(&format!("{:?}", serialized));
        let bytes_key = hash_to_32bit_array(key);

        self.mining_tree.insert(bytes_key);
        self.mining_tree.commit();

        let entry = LedgerEntry {
            key: bytes_key,
            value: serialized,
            proof: None,
            version: 0,
        };

        self.entries.insert(bytes_key, entry);
        bytes_key
    }

    pub fn store_account(&mut self, account: Wallet) -> [u8; 32] {
        let mut serialized = vec![0u8; 256];
        encode_into_slice(&account, &mut serialized, config::standard()).unwrap();
        let key = transform(&format!("{:?}", serialized));
        let bytes_key = hash_to_32bit_array(key);

        self.accounts_tree.insert(bytes_key);
        self.accounts_tree.commit();

        let entry = LedgerEntry {
            key: bytes_key,
            value: serialized,
            proof: None,
            version: 0,
        };

        self.entries.insert(bytes_key, entry);
        bytes_key
    }

    pub fn store_block(&mut self, block: Block) -> [u8; 32] {
        let mut serialized = vec![0u8; 256];
        encode_into_slice(&block, &mut serialized, config::standard()).unwrap();
        let key = transform(&format!("{:?}", serialized));
        let bytes_key = hash_to_32bit_array(key);

        self.blocks_tree.insert(bytes_key);
        self.blocks_tree.commit();

        let entry = LedgerEntry {
            key: bytes_key,
            value: serialized,
            proof: None,
            version: 0,
        };

        self.entries.insert(bytes_key, entry);
        bytes_key
    }

    pub fn generate_proofs(&mut self, tree_identifier: &str) {
        let tree = match tree_identifier {
            "mining" => &self.mining_tree,
            "accounts" => &self.accounts_tree,
            "blocks" => &self.blocks_tree,
            _ => return,
        };

        let leaves = tree.get_leaves();
        if leaves.is_empty() {
            return;
        }

        for (i, leaf) in leaves.iter().enumerate() {
            if let Some(entry) = self.entries.get_mut(leaf) {
                let proof_indices = vec![i];
                let proof_data = tree.generate_proof_bytes(&[i]);

                entry.proof = Some(LedgerProof {
                    tree_identifier: tree_identifier.to_string(),
                    proof_indices,
                    proof_data,
                });
                entry.version += 1;
            }
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

    pub fn decode_difficulty(
        value: &[u8],
    ) -> Result<DifficultyUpdate, bincode::error::DecodeError> {
        decode_from_slice::<DifficultyUpdate, _>(value, bincode::config::standard())
            .map(|(value, _)| value)
    }

    pub fn decode_account(value: &[u8]) -> Result<Wallet, bincode::error::DecodeError> {
        decode_from_slice::<Wallet, _>(value, bincode::config::standard()).map(|(value, _)| value)
    }

    pub fn decode_block(value: &[u8]) -> Result<Block, bincode::error::DecodeError> {
        decode_from_slice::<Block, _>(value, bincode::config::standard()).map(|(value, _)| value)
    }

    fn determine_value_type(&self, value: &[u8]) -> String {
        if let Ok(difficulty) = Ledger::decode_difficulty(value) {
            format!(
                "{{current:{}, previous:{}, difference:{}}}",
                difficulty.current, difficulty.previous, difficulty.difference
            )
        } else if let Ok(account) = Ledger::decode_account(value) {
            format!("{{address:{:?}, deleted:{}}}", account.address, false)
        } else if let Ok(block) = Ledger::decode_block(value) {
            format!(
                "{{hash:{:?}, height:{}, transactions:{:?}}}",
                block.hash, block.height, block.transactions
            )
        } else {
            format!("{{bytes:{:?}}}", value)
        }
    }
}
