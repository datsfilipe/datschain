use crate::{
    client::peer::receive_from_peer,
    storage::{ledger::Ledger, level_db::Storage},
    utils::encoding::decode_base64_to_string,
};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, error::Error, io, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, Mutex},
    time::{sleep, timeout},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct PeerConnection {
    pub framed: Framed<TcpStream, LengthDelimitedCodec>,
    pub addr: SocketAddr,
}

pub struct SharedState {
    pub ledger: Mutex<Ledger>,
    pub storage: Mutex<Storage>,
    pub tx: broadcast::Sender<Bytes>,
    pub peers: Mutex<HashMap<SocketAddr, Framed<TcpStream, LengthDelimitedCodec>>>,
}

async fn handle_connection(
    stream: TcpStream,
    state: Arc<SharedState>,
    peer_addr: SocketAddr,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Handling connection with: {}", peer_addr);
    let codec = LengthDelimitedCodec::new();
    let framed = Framed::new(stream, codec);

    {
        let mut peers = state.peers.lock().await;
        peers.insert(peer_addr, framed);
        println!(
            "Added peer {} to active connections (total: {})",
            peer_addr,
            peers.len()
        );
    }

    let mut rx = state.tx.subscribe();

    let broadcast_fut = async {
        loop {
            match rx.recv().await {
                Ok(msg_bytes) => {
                    let mut peers = state.peers.lock().await;
                    if let Some(framed) = peers.get_mut(&peer_addr) {
                        if let Err(e) = framed.send(msg_bytes.clone()).await {
                            eprintln!("Error sending message to {}: {}", peer_addr, e);
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!("Failed to send to peer {}: {}", peer_addr, e),
                            ));
                        }
                    } else {
                        eprintln!("Peer {} not found in peers map", peer_addr);
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("Peer {} not found", peer_addr),
                        ));
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    println!(
                        "Broadcast channel closed. Stopping connection handling for {}",
                        peer_addr
                    );
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Broadcast channel closed",
                    ));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!(
                        "Connection handler for {} lagged by {} messages.",
                        peer_addr, n
                    );
                }
            }
        }
    };

    let receive_fut = async {
        loop {
            let frame_result = {
                let mut peers = state.peers.lock().await;
                if let Some(framed) = peers.get_mut(&peer_addr) {
                    framed.next().await
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Peer {} not found", peer_addr),
                    ));
                }
            };

            match frame_result {
                Some(Ok(frame)) => match String::from_utf8(frame.to_vec()) {
                    Ok(base64_str) => match decode_base64_to_string(&base64_str) {
                        Ok(message) => {
                            println!("Received from {}: {}", peer_addr, message);
                            if message.starts_with("blocks:")
                                || message.starts_with("accounts:")
                                || message.starts_with("mining:")
                            {
                                let parts: Vec<&str> = message.splitn(2, ':').collect();
                                if parts.len() == 2 {
                                    let identifier = parts[0];
                                    let decoded_inner_data = parts[1].to_string();
                                    if decoded_inner_data.is_empty() {
                                        continue;
                                    }
                                    let mut ledger = state.ledger.lock().await;
                                    let mut storage = state.storage.lock().await;
                                    match receive_from_peer(
                                        decoded_inner_data,
                                        &mut ledger,
                                        &mut storage,
                                        identifier,
                                    )
                                    .await
                                    {
                                        Ok(response) => {
                                            println!(
                                                "Processed message from {}. Response: {}",
                                                peer_addr, response
                                            );
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error processing message from {}: {}",
                                                peer_addr, e
                                            );
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "Invalid message format from {}: {}",
                                        peer_addr, message
                                    );
                                }
                            } else {
                                println!(
                                    "Received non-ledger message from {}: {}",
                                    peer_addr, message
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to decode Base64 content from {}: {}", peer_addr, e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Received invalid UTF-8 frame from {}: {}", peer_addr, e);
                    }
                },
                Some(Err(e)) => {
                    eprintln!("Error reading frame from {}: {}", peer_addr, e);
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to read from peer {}: {}", peer_addr, e),
                    ));
                }
                None => {
                    println!("Connection closed by {}", peer_addr);
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        format!("Connection closed by {}", peer_addr),
                    ));
                }
            }
        }
    };

    let result = tokio::select! {
        r = broadcast_fut => r,
        r = receive_fut => r,
    }?;

    let mut peers = state.peers.lock().await;
    peers.remove(&peer_addr);
    println!(
        "Removed peer {} from active connections (remaining: {})",
        peer_addr,
        peers.len()
    );

    println!("Closing connection with {}", peer_addr);
    result
}

pub async fn broadcast_to_peers(state: &Arc<SharedState>, message: Bytes) {
    let _ = state.tx.send(message.clone()).map_err(|e| {
        eprintln!("Broadcast on channel failed: {}", e);
    });

    let mut peers = state.peers.lock().await;
    let peer_count = peers.len();
    println!("Broadcasting to {} connected peers", peer_count);

    let peer_addrs: Vec<SocketAddr> = peers.keys().cloned().collect();
    for peer_addr in peer_addrs {
        if let Some(framed) = peers.get_mut(&peer_addr) {
            match framed.send(message.clone()).await {
                Ok(_) => {
                    println!("Successfully broadcast to peer: {}", peer_addr);
                }
                Err(e) => {
                    eprintln!("Failed to send to peer {}: {}", peer_addr, e);
                }
            }
        }
    }
}

pub async fn start_network_listener(addr: &str, state: Arc<SharedState>) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Network listener started on {}", addr);
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                println!("Accepted connection from: {}", peer_addr);
                let state_clone = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state_clone, peer_addr).await {
                        eprintln!("Error handling connection from {}: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn start_network_connector(addr: &str, state: Arc<SharedState>) -> io::Result<()> {
    println!("Attempting to connect to {}", addr);
    match TcpStream::connect(addr).await {
        Ok(stream) => {
            let peer_addr = stream
                .peer_addr()
                .unwrap_or_else(|_| addr.parse().expect("Invalid addr"));
            println!("Successfully connected to {}", peer_addr);
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, state, peer_addr).await {
                    eprintln!("Error handling outbound connection to {}: {}", peer_addr, e);
                }
            });
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", addr, e);
            Err(e)
        }
    }
}

pub async fn connect_to_peers(state: Arc<SharedState>, peer_addresses: Vec<String>) {
    // TODO: Implement peer removal by maximum number of failures
    for addr in peer_addresses {
        let state_clone = Arc::clone(&state);
        let addr_clone = addr.clone();

        tokio::spawn(async move {
            let mut attempt = 0;
            let mut delay = Duration::from_secs(15);

            while attempt < 5 {
                match timeout(Duration::from_secs(10), TcpStream::connect(&addr_clone)).await {
                    Ok(Ok(stream)) => {
                        let peer_addr = stream.peer_addr().unwrap_or_else(|_| {
                            addr_clone.parse().expect("Invalid peer address format")
                        });

                        tokio::spawn(handle_connection(
                            stream,
                            Arc::clone(&state_clone),
                            peer_addr,
                        ));
                        break;
                    }
                    Ok(Err(e)) => {
                        eprintln!("Failed to connect to {}: {}", addr_clone, e);
                    }
                    Err(_) => {
                        eprintln!("Connection to {} timed out after {:?}", addr_clone, delay);
                    }
                }

                attempt += 1;
                delay += Duration::from_secs(10);
                sleep(delay).await;
            }
        });
    }
}
