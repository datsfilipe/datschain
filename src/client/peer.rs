use crate::{
    storage::{
        ledger::{DeserializedLedgerValue, Ledger},
        level_db::Storage,
    },
    utils::conversion::to_hex,
};
use std::error::Error;

async fn process_peer_state(
    ledger: &mut Ledger,
    storage: &mut Storage,
    data: String,
    identifier: &str,
) -> Result<[u8; 32], Box<dyn Error + Send + Sync>> {
    let new_state: DeserializedLedgerValue =
        match serde_json::from_str::<DeserializedLedgerValue>(&data) {
            Ok(value) => value,
            Err(e) => {
                return Err(format!(
                    "Failed to parse JSON value from peer: {}. Data: '{}'",
                    e, data
                )
                .into())
            }
        };

    let calculated_key = ledger.get_key(&new_state.value);
    if ledger.entries.contains_key(&calculated_key) {
        return Err(format!(
            "{} with calculated key {} already exists in ledger",
            identifier,
            to_hex(&calculated_key)
        )
        .into());
    }

    println!(
        "Processing new state for identifier '{}' with calculated key {}",
        identifier,
        to_hex(&calculated_key)
    );

    match ledger
        .sync_client_block(calculated_key, new_state.value, identifier, storage)
        .await
    {
        Some(_) => {
            println!(
                "Successfully committed peer state ({}) to ledger with key {}",
                identifier,
                to_hex(&calculated_key)
            );
            Ok(calculated_key)
        }
        None => {
            eprintln!(
                "Failed to commit peer state ({}) to ledger for key {}",
                identifier,
                to_hex(&calculated_key)
            );
            Err(format!("Failed to commit {} state to ledger", identifier).into())
        }
    }
}

async fn handle_peer_message(
    ledger: &mut Ledger,
    storage: &mut Storage,
    decoded_inner_data: String,
    identifier: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match process_peer_state(ledger, storage, decoded_inner_data, identifier).await {
        Ok(key) => Ok(format!("Data ({}) accepted: {}", identifier, to_hex(&key))),
        Err(e) => Err(format!("Rejected {} state: {}", identifier, e).into()),
    }
}

pub async fn receive_from_peer(
    decoded_inner_data: String,
    ledger: &mut Ledger,
    storage: &mut Storage,
    identifier: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    println!("Received message from peer: {}", decoded_inner_data);

    handle_peer_message(ledger, storage, decoded_inner_data, identifier).await
}
