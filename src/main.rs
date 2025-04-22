use bytes::Bytes;
use chain::blockchain::Blockchain;
use client::network::connect_to_peers;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex, RwLock};

mod account;
mod chain;
mod client;
mod cryptography;
mod storage;
mod utils;

#[tokio::main]
async fn main() {
    let (broadcast_tx, _) = broadcast::channel::<Bytes>(100);
    let state = Arc::new(client::network::SharedState {
        ledger: Mutex::new(storage::ledger::Ledger::new()),
        storage: Mutex::new(storage::level_db::Storage::new(
            &utils::env::get_database_path(),
        )),
        tx: broadcast_tx,
        peers: RwLock::new(HashMap::new()),
        seen_messages: Mutex::new(std::collections::HashSet::new()),
    });

    let routes = client::http::create_connect_endpoint(Arc::clone(&state));
    let blockchain = Arc::new(Mutex::new(chain::blockchain::Blockchain::new(0)));
    let block_manager = Arc::new(Mutex::new(chain::block_manager::BlockManager::new(500)));

    tokio::spawn(async move {
        let addr = utils::env::get_api_addr();
        println!("HTTP server listening on {}", &addr);
        warp::serve(routes).run(addr).await;
    });

    let addr = utils::env::get_listen_addr();
    let listener = tokio::spawn({
        let state_clone = Arc::clone(&state);
        let addr_clone = addr.clone();
        async move { client::network::start_network_listener(&addr_clone, state_clone).await }
    });

    let connector = tokio::spawn({
        let state_clone = Arc::clone(&state);
        let addr_clone = addr.clone();
        async move { client::network::start_network_connector(&addr_clone, state_clone, addr).await }
    });

    let mining =
        Blockchain::start_mining_service_async(blockchain, block_manager, Arc::clone(&state));

    let retry_service = tokio::spawn({
        let state_clone = Arc::clone(&state);
        async move {
            let peer_addresses = utils::env::get_peer_addresses();
            if !peer_addresses.is_empty() {
                println!("Attempting to connect to known peers: {:?}", peer_addresses);
                connect_to_peers(state_clone, peer_addresses).await;
            }
        }
    });

    let _ =
        tokio::try_join!(listener, connector, mining, retry_service).expect("Failed to join tasks");
}
