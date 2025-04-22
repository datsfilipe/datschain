use std::error::Error;

use crate::storage::ledger::DeserializedLedgerValue;
use crate::storage::{ledger::Ledger, level_db::Storage};
use crate::utils::conversion::to_hex;
use crate::utils::encoding::decode_base64_to_string;

pub async fn process_peer_state(
    ledger: &mut Ledger,
    storage: &mut Storage,
    data: String,
    identifier: &str,
) -> Result<[u8; 32], Box<dyn Error + Send + Sync>> {
    let new_state = match serde_json::from_str::<DeserializedLedgerValue>(&data) {
        Ok(value) => value,
        Err(e) => return Err(format!("Failed to parse JSON value: {}", e).into()),
    };

    let exists = ledger
        .entries
        .iter()
        .any(|(_, entry)| entry.key == new_state.key.as_bytes());

    if exists {
        return Err(format!("{} already exists in ledger", identifier).into());
    }

    let key = ledger.get_key(&new_state.value);
    match ledger
        .sync_client_block(key, new_state.value, identifier, storage)
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
    data: String,
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
    let decoded =
        decode_base64_to_string(&message).map_err(|e| format!("Error decoding message: {}", e))?;
    handle_peer_message(ledger, storage, decoded, identifier).await
}
