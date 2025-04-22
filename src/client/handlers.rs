use crate::{
    account::wallet::Wallet,
    client::network::{broadcast_to_peers, SharedState},
    storage::ledger::LedgerValue,
    utils::{conversion::to_hex, encoding::encode_string_to_base64},
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::{reject::Rejection, reply::Reply};

#[derive(Deserialize)]
pub struct Request {
    private_key: String,
    public_key: String,
}

#[derive(Serialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub tx_hash: Option<[u8; 32]>,
}

pub async fn process_connect_request(
    state: Arc<SharedState>,
    body: warp::hyper::body::Bytes,
) -> Result<impl Reply, Rejection> {
    println!("Received connect request from client");
    let request_str = match String::from_utf8(body.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            return Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Invalid UTF-8 sequence: {}", e),
                tx_hash: None,
            }))
        }
    };
    let data: Request = match serde_json::from_str(&request_str) {
        Ok(d) => d,
        Err(e) => {
            return Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Deserialize error: {}", e),
                tx_hash: None,
            }))
        }
    };
    let wallet = Wallet::new(
        data.private_key.as_bytes().to_vec(),
        data.public_key.as_bytes().to_vec(),
    );
    let address = wallet.get_address();
    println!("Created wallet with address: {}", address);
    let key = {
        let ledger_guard = state.ledger.lock().await;
        ledger_guard.get_key(&LedgerValue::Accounts(wallet.clone()))
    };
    println!("Generated key for wallet: {}", to_hex(&key));
    let commit_result = {
        let mut ledger_guard = state.ledger.lock().await;
        let mut storage_guard = state.storage.lock().await;
        ledger_guard
            .commit_with_identifier(
                key,
                LedgerValue::Accounts(wallet.clone()),
                "accounts",
                &mut *storage_guard,
            )
            .await
    };
    if commit_result.is_none() {
        println!("Failed to commit wallet to ledger");
        return Ok(warp::reply::json(&Response {
            success: false,
            message: "Failed to commit wallet to ledger".to_string(),
            tx_hash: None,
        }));
    }
    println!("Successfully committed wallet to ledger");
    let entry = {
        let ledger_guard = state.ledger.lock().await;
        ledger_guard.entries.get(&key).cloned()
    };
    let entry = match entry {
        Some(e) => e,
        None => {
            eprintln!(
                "Error: Entry not found in ledger after successful commit for key {}",
                to_hex(&key)
            );
            return Ok(warp::reply::json(&Response {
                success: false,
                message: "Internal error: Failed to find committed entry".to_string(),
                tx_hash: Some(key),
            }));
        }
    };
    let formatted_entry_str = state
        .ledger
        .lock()
        .await
        .format_entry_value(&key, &entry.value);
    let broadcast_message_plain = format!("accounts:{}", formatted_entry_str);
    let broadcast_message_base64 = encode_string_to_base64(&broadcast_message_plain);
    let broadcast_payload = Bytes::from(broadcast_message_base64.into_bytes());

    broadcast_to_peers(&state, broadcast_payload).await;

    println!("Broadcasted account update for address: {}", address);
    let store_result = {
        let mut storage_guard = state.storage.lock().await;
        let formatted_value_for_storage = state
            .ledger
            .lock()
            .await
            .format_entry_value(&key, &entry.value);
        storage_guard.store(&key, formatted_value_for_storage).await
    };
    match store_result {
        Ok(_) => {
            println!(
                "Successfully stored wallet locally for address: {}",
                address
            );
            Ok(warp::reply::json(&Response {
                success: true,
                message: format!("Account created and broadcasted. address: {}", address),
                tx_hash: Some(key),
            }))
        }
        Err(e) => {
            println!("Storage error after commit and broadcast: {}", e);
            Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Account created but failed local storage: {}", e),
                tx_hash: Some(key),
            }))
        }
    }
}
