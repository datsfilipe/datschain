use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::{reject::Rejection, reply::Reply};

use crate::{
    account::wallet::Wallet,
    client::{network::SharedState, sink::safe_send},
    storage::ledger::LedgerValue,
    utils::encoding::decode_base64_to_string,
};

#[derive(Deserialize)]
pub struct Request {
    private_key: String,
    public_key: String,
}

#[derive(Serialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub tx_hash: Option<String>,
}

pub async fn process_connect_request(
    state: Arc<SharedState>,
    request: String,
) -> Result<impl Reply, Rejection> {
    println!("Received connect request from client");

    let decoded = match decode_base64_to_string(&request) {
        Ok(decoded) => decoded,
        Err(e) => {
            return Ok(warp::reply::json(&Response {
                success: false,
                message: e.to_string(),
                tx_hash: None,
            }))
        }
    };

    let data: Request = match serde_json::from_str(&decoded) {
        Ok(data) => data,
        Err(e) => {
            return Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Request body deserialize error: {}", e),
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
    println!("Generated key for wallet");

    let commit_result = {
        let mut ledger_guard = state.ledger.lock().await;
        let mut storage_guard = state.storage.lock().await;
        ledger_guard
            .commit_with_identifier(
                key,
                LedgerValue::Accounts(wallet),
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

    let entry = {
        let ledger_guard = state.ledger.lock().await;
        let entry = ledger_guard.entries.get(&key).cloned();
        match entry {
            Some(entry) => entry,
            None => {
                return Ok(warp::reply::json(&Response {
                    success: false,
                    message: "Failed to find entry in ledger".to_string(),
                    tx_hash: None,
                }))
            }
        }
    };

    match safe_send(&state.sink, &format!("accounts:{:?}", entry)).await {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to send response to client: {}", e);
            return Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Failed to send response to client: {}", e),
                tx_hash: None,
            }));
        }
    }

    let formatted_entry = {
        let ledger_guard = state.ledger.lock().await;
        ledger_guard.format_entry_value(&key, &entry.value)
    };

    let store_result = {
        let mut storage_guard = state.storage.lock().await;
        storage_guard.store(&key, formatted_entry).await
    };

    match store_result {
        Ok(_) => {
            println!("Successfully stored wallet with address: {}", address);
            Ok(warp::reply::json(&Response {
                success: true,
                message: format!("address: {}", address),
                tx_hash: None,
            }))
        }
        Err(e) => {
            println!("Storage error: {}", e);
            Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Storage error: {}", e),
                tx_hash: None,
            }))
        }
    }
}
