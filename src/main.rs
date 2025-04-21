use chain::blockchain::Blockchain;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

mod account;
mod chain;
mod client;
mod cryptography;
mod storage;
mod utils;

fn get_listen_addr() -> String {
    std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string())
}

fn get_api_addr() -> SocketAddr {
    std::env::var("API_ADDR")
        .ok()
        .and_then(|addr_str| addr_str.parse::<SocketAddr>().ok())
        .unwrap_or_else(|| {
            eprintln!("Invalid or unset LISTEN_ADDR, defaulting to 127.0.0.1:3001");
            "127.0.0.1:3001"
                .parse()
                .expect("Default address is invalid")
        })
}

fn get_database_path() -> String {
    std::env::var("DATABASE_PATH").unwrap_or_else(|_| "/tmp/ledger".to_string())
}

#[tokio::main]
async fn main() {
    let blockchain = Arc::new(Mutex::new(chain::blockchain::Blockchain::new(0)));
    let block_manager = Arc::new(Mutex::new(chain::block_manager::BlockManager::new(500)));

    let state = Arc::new(client::network::SharedState {
        ledger: Mutex::new(storage::ledger::Ledger::new()),
        storage: Mutex::new(storage::level_db::Storage::new(&get_database_path())),
    });
    let routes = client::http::create_connect_endpoint(Arc::clone(&state));

    tokio::spawn(async move {
        let addr = get_api_addr();
        println!("HTTP server listening on {}", &addr);
        warp::serve(routes).run(addr).await;
    });

    let addr = get_listen_addr();
    let listener = tokio::spawn({
        let addr = addr.clone();
        async move { client::network::start_network_listener(&addr, state).await }
    });
    let connector = tokio::spawn({
        let addr = addr.clone();
        async move { client::network::start_network_connector(&addr).await }
    });
    let mining = Blockchain::start_mining_service_async(blockchain, block_manager);

    let _ = tokio::try_join!(listener, connector, mining).expect("Failed to join");
}
