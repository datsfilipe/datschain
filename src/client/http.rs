use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

use crate::account::wallet::Wallet;
use crate::client::network::SharedState;

#[derive(Deserialize)]
pub struct SendDataRequest {
    pub data: String,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    private_key: String,
    public_key: String,
}

#[derive(Serialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub tx_hash: Option<String>,
}

async fn process_connect_request(
    state: Arc<SharedState>,
    request: ConnectRequest,
) -> Result<impl Reply, Rejection> {
    let wallet = Wallet::new(
        request.private_key.as_bytes().to_vec(),
        request.public_key.as_bytes().to_vec(),
    );
    let address = wallet.get_address();
    let key = state.ledger.lock().await.store_account(wallet);
    let formatted_entry = state.ledger.lock().await.format_entry(&key);

    state
        .storage
        .lock()
        .await
        .store(&key, formatted_entry)
        .await
        .unwrap();

    Ok(warp::reply::json(&Response {
        success: true,
        message: format!("address: {}", address),
        tx_hash: None,
    }))
}

pub fn create_connect_endpoint(
    state: Arc<SharedState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "connect")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::any().map(move || Arc::clone(&state)))
        .and_then(|request, state| process_connect_request(state, request))
        .with(warp::cors().allow_any_origin())
}
