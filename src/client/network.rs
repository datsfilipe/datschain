use bytes::Bytes; 
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, error::Error, io, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc, Mutex, RwLock},
    time::{sleep, timeout},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use sha2::{Digest, Sha256}; 

use crate::{
    client::peer::receive_from_peer,
    storage::{ledger::Ledger, level_db::Storage},
    utils::{conversion::from_hex, encoding::{decode_base64_to_string, encode_string_to_base64}, env::get_listen_addr},
};

fn calculate_message_id(payload: &Bytes) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.finalize().into()
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

pub struct SharedState {
    pub ledger: Mutex<Ledger>,   
    pub storage: Mutex<Storage>, 
    pub tx: broadcast::Sender<Bytes>,
    pub peers: RwLock<HashMap<SocketAddr, mpsc::Sender<Bytes>>>,
    pub seen_messages: Mutex<std::collections::HashSet<[u8; 32]>>,
}

async fn handle_connection(
    stream: TcpStream,
    state: Arc<SharedState>,
    peer_addr: SocketAddr,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Handling connection with: {}", peer_addr);
    
    let codec = LengthDelimitedCodec::new();
    let framed = Framed::new(stream, codec);
    let (mut writer, mut reader) = framed.split();
    let (peer_tx, mut peer_rx) = mpsc::channel::<Bytes>(100); 

    {
        let mut peers = state.peers.write().await; 
                                                   
        peers.insert(peer_addr, peer_tx.clone());
        println!(
            "Added peer {} to active connections (total: {})",
            peer_addr,
            peers.len()
        );
    } 
    
    let state_clone_for_broadcast = Arc::clone(&state);
    let state_clone_for_receive = Arc::clone(&state);
    let peer_addr_clone_for_send = peer_addr;
    let peer_addr_clone_for_broadcast = peer_addr;
    let peer_addr_clone_for_receive = peer_addr;

    tokio::spawn(async move {
        println!("Send task started for {}", peer_addr_clone_for_send);
        while let Some(msg_bytes_hex_encoded) = peer_rx.recv().await {
            if let Err(e) = writer.send(msg_bytes_hex_encoded).await {
                eprintln!(
                    "Error sending message to {}: {}",
                    peer_addr_clone_for_send, e
                );
                break; 
            }
        }
        println!("Send task for {} finished", peer_addr_clone_for_send);
    });

    let mut broadcast_rx = state_clone_for_broadcast.tx.subscribe();
    tokio::spawn(async move {
        println!(
            "Broadcast forward task started for {}",
            peer_addr_clone_for_broadcast
        );
        loop {
            match broadcast_rx.recv().await {
                Ok(msg_bytes_hex_encoded) => {
                    if let Err(_) = peer_tx.send(msg_bytes_hex_encoded).await {
                        println!(
                            "Peer send channel closed for {}. Stopping broadcast forwarding.",
                            peer_addr_clone_for_broadcast
                        );
                        break; 
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    println!(
                        "Global broadcast channel closed. Stopping forwarding for {}",
                        peer_addr_clone_for_broadcast
                    );
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!(
                        "Broadcast forwarder for {} lagged by {} messages.",
                        peer_addr_clone_for_broadcast, n
                    );
                }
            }
        }
        println!(
            "Broadcast forward task for {} finished",
            peer_addr_clone_for_broadcast
        );
    });
    
    println!("Receive task started for {}", peer_addr_clone_for_receive);
    let receive_result: Result<(), Box<dyn Error + Send + Sync>> = loop {
        let frame_result = reader.next().await;

        match frame_result {
            Some(Ok(frame_bytes_hex_encoded)) => {
                let hex_string = match String::from_utf8(frame_bytes_hex_encoded.to_vec()) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!(
                            "Received invalid UTF-8 hex string from {}: {}",
                            peer_addr_clone_for_receive, e
                        );
                        continue; 
                    }
                };

                let raw_bytes = match from_hex(&hex_string) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        eprintln!(
                            "Failed to decode hex string from {}: {}",
                            peer_addr_clone_for_receive, e
                        );
                        continue; 
                    }
                };

                if raw_bytes.len() < 32 {
                    eprintln!(
                        "Received message from {} too short ({} bytes) to contain message ID.",
                        peer_addr_clone_for_receive,
                        raw_bytes.len()
                    );
                    continue; 
                }
                
                let (msg_id_slice, payload_bytes_slice) = raw_bytes.split_at(32);
                let msg_id: [u8; 32] = msg_id_slice
                    .try_into()
                    .expect("Slice length mismatch for message ID"); 
                let msg_payload_bytes = Bytes::from(payload_bytes_slice.to_vec()); 

                let is_seen = {
                    let mut seen_messages = state_clone_for_receive.seen_messages.lock().await;
                    if seen_messages.contains(&msg_id) {
                        true 
                    } else {
                        seen_messages.insert(msg_id); 
                        false 
                    }
                }; 

                if is_seen {
                    println!(
                        "Received duplicate message {} from {}. Skipping processing and relay.",
                        to_hex(&msg_id),
                        peer_addr_clone_for_receive
                    );
                    continue; 
                }

                println!(
                    "Received new message {} from {}",
                    to_hex(&msg_id),
                    peer_addr_clone_for_receive
                );

                match String::from_utf8(msg_payload_bytes.to_vec()) {
                    Ok(base64_str) => match decode_base64_to_string(&base64_str) {
                        Ok(message) => {
                            println!("Decoded message content: {}", message); 
                            
                            if message.starts_with("blocks:")
                                || message.starts_with("accounts:")
                                || message.starts_with("mining:")
                            {
                                let parts: Vec<&str> = message.splitn(2, ':').collect();
                                if parts.len() == 2 {
                                    let identifier = parts[0];
                                    let decoded_inner_data = parts[1].to_string(); 
                                    if decoded_inner_data.is_empty() {
                                        println!("Received message with empty inner data from {}. Skipping processing.", peer_addr_clone_for_receive);
                                        continue; 
                                    }
                                    
                                    let mut ledger = state_clone_for_receive.ledger.lock().await;
                                    let mut storage = state_clone_for_receive.storage.lock().await;
                                    let should_relay: bool;

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
                                                "Processed unique message {} from {}. Response: {}",
                                                to_hex(&msg_id), 
                                                peer_addr_clone_for_receive,
                                                response
                                            );
                                            should_relay = true; 
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error processing unique message {} from {}: {}",
                                                to_hex(&msg_id), 
                                                peer_addr_clone_for_receive,
                                                e
                                            );
                                            should_relay = false;
                                        }
                                    } 
                                    
                                    if should_relay {
                                        let peers_map = state_clone_for_receive.peers.read().await; 
                                        let recipients: Vec<mpsc::Sender<Bytes>> = peers_map
                                            .iter()
                                            .filter(|(addr, _)| {
                                                **addr != peer_addr_clone_for_receive
                                            })
                                            .map(|(_, sender)| sender.clone()) 
                                            .collect();
                                        drop(peers_map); 

                                        println!(
                                            "Relaying unique message {} from {} to {} other peers",
                                            to_hex(&msg_id), 
                                            peer_addr_clone_for_receive,
                                            recipients.len()
                                        );

                                        let message_bytes_to_relay = frame_bytes_hex_encoded; 
                                        for recipient_tx in recipients {
                                            let msg_clone = message_bytes_to_relay.clone(); 
                                            tokio::spawn(async move {
                                                if let Err(_) =
                                                    recipient_tx.send(msg_clone.into()).await
                                                {
                                                }
                                            });
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "Invalid message format from {}: {}",
                                        peer_addr_clone_for_receive, message
                                    );
                                }
                            } else {
                                println!(
                                    "Received non-ledger message from {}: {}",
                                    peer_addr_clone_for_receive, message
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to decode Base64 content from message from {}: {}",
                                peer_addr_clone_for_receive, e
                            );
                        }
                    },
                    Err(e) => {
                        eprintln!(
                            "Received message payload with invalid UTF-8 content from {}: {}",
                            peer_addr_clone_for_receive, e
                        );
                    }
                }
            }
            Some(Err(e)) => {
                eprintln!(
                    "Error reading frame from {}: {}",
                    peer_addr_clone_for_receive, e
                );
                
                break Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Failed to read from peer {}: {}",
                        peer_addr_clone_for_receive, e
                    ),
                )
                .into());
            }
            None => {
                println!("Connection closed by {}", peer_addr_clone_for_receive);
                break Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    format!("Connection closed by {}", peer_addr_clone_for_receive),
                )
                .into());
            }
        }
    }; 

    println!(
        "Receive task for {} finished with result: {:?}",
        peer_addr, receive_result
    );

    {
        let mut peers = state.peers.write().await; 
        peers.remove(&peer_addr); 
        println!(
            "Removed peer {} from active connections (remaining: {})",
            peer_addr,
            peers.len()
        );
    } 

    receive_result
}




pub async fn broadcast_to_peers(state: &Arc<SharedState>, payload_string: String) {
    let base64_payload_bytes = Bytes::from(encode_string_to_base64(&payload_string).into_bytes());
    let msg_id = calculate_message_id(&base64_payload_bytes);
    let mut raw_message_bytes = Vec::with_capacity(32 + base64_payload_bytes.len());
    raw_message_bytes.extend_from_slice(&msg_id);
    raw_message_bytes.extend_from_slice(&base64_payload_bytes);
    let hex_encoded_message_string = to_hex(&raw_message_bytes);
    let serialized_msg_bytes_hex_encoded = Bytes::from(hex_encoded_message_string.into_bytes()); 

    let is_seen = {
        let mut seen_messages = state.seen_messages.lock().await; 
        if seen_messages.contains(&msg_id) {
            true 
        } else {
            seen_messages.insert(msg_id); 
            false
        }
    }; 

    if is_seen {
        eprintln!(
            "Anomaly: Attempted to broadcast already seen message ID: {}",
            to_hex(&msg_id)
        );
    }

    let receiver_count = state.tx.receiver_count();
    println!(
        "Broadcasting locally originated message {} ({} bytes payload, {} hex bytes) to {} subscribers via channel",
        to_hex(&msg_id), 
        base64_payload_bytes.len(), 
        serialized_msg_bytes_hex_encoded.len(), 
        receiver_count
    );
    
    let _ = state
        .tx
        .send(serialized_msg_bytes_hex_encoded)
        .map_err(|e| {
            eprintln!(
                "Broadcast on channel failed for message {}: {}",
                to_hex(&msg_id),
                e
            );
        });
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

pub async fn start_network_connector(
    addr: &str,
    state: Arc<SharedState>,
    listen_addr: String, 
) -> io::Result<()> {
    if addr == listen_addr {
        println!("Skipping connection to self: {}", addr);
        return Ok(());
    }

    println!("Attempting to connect to {}", addr);
    match TcpStream::connect(addr).await {
        Ok(stream) => {
            let peer_addr = stream
                .peer_addr()
                .unwrap_or_else(|_| addr.parse().expect("Invalid addr format")); 

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
    let listen_addr = get_listen_addr();
    for addr in peer_addresses {
        if addr == listen_addr {
            println!("Skipping connection to self: {}", addr);
            continue;
        }
        
        let state_clone = Arc::clone(&state);
        let addr_clone = addr.clone();
        
        tokio::spawn(async move {
            let mut attempt = 0;
            let max_attempts = 5; 
            let mut delay = Duration::from_secs(15); 

            while attempt < max_attempts {
                println!(
                    "Attempt {}/{} to connect to {}",
                    attempt + 1,
                    max_attempts,
                    addr_clone
                );
                
                match timeout(Duration::from_secs(10), TcpStream::connect(&addr_clone)).await {
                    Ok(Ok(stream)) => {
                        let peer_addr = stream.peer_addr().unwrap_or_else(|_| {
                            addr_clone.parse().expect("Invalid peer address format")
                            
                        });

                        println!("Successfully connected to {}", peer_addr);
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
                        
                        eprintln!(
                            "Connection to {} timed out after {:?}",
                            addr_clone,
                            Duration::from_secs(10)
                        ); 
                           
                    }
                }

                attempt += 1;
                if attempt < max_attempts {
                    delay += Duration::from_secs(10); 
                    println!("Retrying connection to {} in {:?}", addr_clone, delay);
                    sleep(delay).await; 
                }
            }
            
            if attempt == max_attempts {
                eprintln!(
                    "Failed to connect to {} after {} attempts",
                    addr_clone, max_attempts
                );
            }
        });
    }
}
