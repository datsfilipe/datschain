mod client;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";

    let listener_handle = tokio::spawn(async move {
        if let Err(e) = client::network::start_network_listener(addr).await {
            eprintln!("Listener failed: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let connector_handle = tokio::spawn(async move {
        if let Err(e) = client::network::start_network_connector(addr).await {
            eprintln!("Connector failed: {}", e);
        }
    });

    let _ = tokio::join!(listener_handle, connector_handle);
    println!("Exiting main (likely because listener or connector task ended unexpectedly).");
}
