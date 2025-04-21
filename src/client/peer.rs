use std::error::Error;

use crate::storage::{
    ledger::{DecodedData, Ledger},
    level_db::Storage,
};
use crate::utils::conversion::to_hex;

pub async fn process_peer_state(
    ledger: &mut Ledger,
    storage: &mut Storage,
    data: Vec<u8>,
    identifier: &str,
) -> Result<[u8; 32], Box<dyn Error + Send + Sync>> {
    let new_state: DecodedData = match ledger.decode_value(&data) {
        Ok(block) => block,
        Err(e) => return Err(format!("Failed to decode block: {}", e).into()),
    };

    let exists = ledger
        .entries
        .iter()
        .any(|(_, entry)| ledger.encode_value(&entry.value) == ledger.encode_value(&new_state));

    if exists {
        return Err(format!("{} already exists in ledger", identifier).into());
    }

    let key = ledger.get_key(&new_state);
    match ledger
        .sync_client_block(key, data, identifier, storage)
        .await
    {
        Some(_) => {
            println!("Successfully committed block to ledger");
            Ok(key)
        }
        None => return Err(format!("Failed to commit block to ledger").into()),
    }
}

pub async fn handle_peer_message(
    ledger: &mut Ledger,
    storage: &mut Storage,
    data: Vec<u8>,
    identifier: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match process_peer_state(ledger, storage, data, identifier).await {
        Ok(key) => Ok(format!("Data accepted: {}", to_hex(&key))),
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
