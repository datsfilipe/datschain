use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::{reject::Rejection, reply::Reply};

use crate::{
    account::wallet::Wallet,
    client::{network::SharedState, sink::safe_send},
    storage::ledger::DecodedData,
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
    request: Request,
) -> Result<impl Reply, Rejection> {
    println!("Received connect request from client");

    let wallet = Wallet::new(
        request.private_key.as_bytes().to_vec(),
        request.public_key.as_bytes().to_vec(),
    );
    let address = wallet.get_address();
    println!("Created wallet with address: {}", address);

    let key = {
        let ledger_guard = state.ledger.lock().await;
        ledger_guard.get_key(&DecodedData::Accounts(wallet.clone()))
    };
    println!("Generated key for wallet");

    let commit_result = {
        let mut ledger_guard = state.ledger.lock().await;
        let mut storage_guard = state.storage.lock().await;
        ledger_guard
            .commit_with_identifier(key, wallet.to_vec(), "accounts", &mut *storage_guard)
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
        ledger_guard.entries.get(&key).cloned()
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
        ledger_guard.format_entry(&key)
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
