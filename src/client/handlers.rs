use crate::{
    account::wallet::Wallet,
    client::network::{broadcast_to_peers, to_hex, SharedState},
    storage::ledger::LedgerValue,
};
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
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
            eprintln!("Invalid UTF-8 sequence in request body: {}", e);
            return Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Invalid UTF-8 sequence: {}", e),
                tx_hash: None,
            }));
        }
    };

    let data: Request = match serde_json::from_str(&request_str) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Deserialize error for request body: {}", e);
            return Ok(warp::reply::json(&Response {
                success: false,
                message: format!("Deserialize error: {}", e),
                tx_hash: None,
            }));
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
        println!("Failed to commit wallet to ledger locally");
        return Ok(warp::reply::json(&Response {
            success: false,
            message: "Failed to commit wallet to ledger".to_string(),
            tx_hash: Some(key),
        }));
    }
    println!("Successfully committed wallet to ledger locally");

    let store_result: Result<(), Box<dyn StdError + Send + Sync>> = {
        let mut storage_guard = state.storage.lock().await;

        let formatted_value_for_storage = {
            let mut ledger_guard = state.ledger.lock().await;
            let value = ledger_guard
                .entries
                .get(&key)
                .expect("Entry disappeared after commit!")
                .value
                .clone();
            ledger_guard.format_entry_value(&key, &value)
        };

        storage_guard.store(&key, formatted_value_for_storage).await
    };

    match store_result {
        Ok(_) => println!("Successfully stored wallet locally"),
        Err(e) => eprintln!("Failed to store wallet locally after commit: {}", e),
    }

    let formatted_entry_value_string = {
        let mut ledger_guard = state.ledger.lock().await;

        let value = &ledger_guard
            .entries
            .get(&key)
            .expect("Entry disappeared after commit!")
            .value
            .clone();
        ledger_guard.format_entry_value(&key, &value)
    };

    let broadcast_payload_string = format!("accounts:{}", formatted_entry_value_string);

    broadcast_to_peers(&state, broadcast_payload_string).await;

    println!("Triggered broadcast for account update: {}", to_hex(&key));

    Ok(warp::reply::json(&Response {
        success: true,
        message: format!(
            "Account created and broadcast triggered. Key: {}",
            to_hex(&key)
        ),
        tx_hash: Some(key),
    }))
}
