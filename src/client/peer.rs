use bincode::{config, encode_into_slice};
use std::error::Error;

use crate::storage::{
    ledger::{self, Ledger},
    level_db::Storage,
};
use crate::utils::conversion::to_hex;

pub async fn process_peer_state(
    ledger: &mut Ledger,
    storage: &mut Storage,
    block_data: &[u8],
    identifier: &str,
) -> Result<[u8; 32], Box<dyn Error + Send + Sync>> {
    let new_state = match Ledger::decode_block(block_data) {
        Ok(block) => block,
        Err(e) => return Err(format!("Failed to decode block: {}", e).into()),
    };

    let exists = ledger.entries.iter().any(|(_, entry)| {
        if let Ok(existing) = Ledger::decode_block(&entry.value) {
            existing.hash == new_state.hash
        } else {
            false
        }
    });

    if exists {
        return Err(format!("{} already exists in ledger", identifier).into());
    }

    let mut serialized = vec![0u8; 256];
    encode_into_slice(&new_state, &mut serialized, config::standard())
        .map_err(|e| format!("Failed to serialize {}: {}", identifier, e))?;

    let bytes_key: [u8; 32] = new_state.hash.try_into().unwrap();
    ledger.commit_with_identifier(&identifier, bytes_key).await;

    let entry = ledger::LedgerEntry {
        key: bytes_key,
        value: serialized,
        proof: None,
        version: 0,
    };

    ledger.entries.insert(bytes_key.try_into().unwrap(), entry);
    ledger.generate_proofs(&identifier);

    if !ledger.verify_entry(&bytes_key) {
        ledger.entries.remove(&bytes_key);
        ledger.rollback_with_identifier(&identifier, bytes_key);
        return Err("Block verification failed".into());
    }

    let formatted_entry = ledger.format_entry(&bytes_key);
    if let Err(e) = storage.store(&bytes_key, formatted_entry).await {
        ledger.entries.remove(&bytes_key);
        return Err(format!("Failed to store block in database: {}", e).into());
    }

    println!("Successfully processed peer block: {}", to_hex(&bytes_key));
    Ok(bytes_key)
}

pub async fn handle_peer_message(
    ledger: &mut Ledger,
    storage: &mut Storage,
    block_data: Vec<u8>,
    identifier: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match process_peer_state(ledger, storage, &block_data, identifier).await {
        Ok(block_key) => Ok(format!("Block accepted: {}", to_hex(&block_key))),
        Err(e) => Err(format!("Rejected block: {}", e).into()),
    }
}

pub async fn receive_from_peer(
    message: String,
    ledger: &mut Ledger,
    storage: &mut Storage,
    identifier: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let engine = base64::engine::general_purpose::STANDARD;
    let decoded = base64::Engine::decode(&engine, message)
        .map_err(|e| format!("Failed to decode base64 message: {}", e))?;

    handle_peer_message(ledger, storage, decoded, identifier).await
}
