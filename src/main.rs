mod account;
mod chain;
mod client;
mod cryptography;
mod storage;
mod utils;

fn get_listen_addr() -> String {
    std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string())
}

#[tokio::main]
async fn main() {
    let listener_handle = tokio::spawn(async move {
        if let Err(e) = client::network::start_network_listener(&get_listen_addr()).await {
            eprintln!("Listener failed: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let connector_handle = tokio::spawn(async move {
        if let Err(e) = client::network::start_network_connector(&get_listen_addr()).await {
            eprintln!("Connector failed: {}", e);
        }
    });

    let _ = tokio::join!(listener_handle, connector_handle);
    println!("Exiting main (likely because listener or connector task ended unexpectedly).");
}
