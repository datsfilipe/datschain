use base64::{engine, Engine};
use std::error::Error;
use std::sync::Arc;
use tokio::io::{self, AsyncReadExt, BufReader, ReadHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::client::peer::receive_from_peer;
use crate::client::sink::{safe_send, MessageSink, TcpSink};
use crate::storage::{ledger::Ledger, level_db::Storage};
use crate::utils::encoding::decode_from_base64;

pub struct SharedState {
    pub ledger: Mutex<Ledger>,
    pub storage: Mutex<Storage>,
    pub sink: Mutex<Option<Arc<dyn MessageSink>>>,
}

async fn read_message(reader: &mut BufReader<ReadHalf<TcpStream>>) -> io::Result<Option<String>> {
    match reader.read_u32().await {
        Ok(len) => {
            if len == 0 {
                return Ok(None);
            }

            let mut buffer = vec![0u8; len as usize];
            reader.read_exact(&mut buffer).await?;

            let base64_str = match String::from_utf8(buffer) {
                Ok(str) => str,
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid UTF-8 sequence in base64 data",
                    ))
                }
            };

            match decode_from_base64(&base64_str) {
                Ok(str) => Ok(Some(str)),
                Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
            }
        }
        Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(e),
    }
}

async fn handle_server_connection(
    stream: TcpStream,
    state: Arc<SharedState>,
) -> Result<(), Box<dyn Error>> {
    let peer_addr = stream.peer_addr()?;
    println!("Server: Connection established with: {}", peer_addr);

    let (reader, _) = io::split(stream);
    let mut reader = BufReader::new(reader);

    if let Some(sink) = &*state.sink.lock().await {
        sink.send("Hello from server!").await?;
    }
    println!("Server: Sent greeting to {}", peer_addr);

    loop {
        match read_message(&mut reader).await {
            Ok(Some(message)) => {
                println!("Server received from {}: {}", peer_addr, message);

                if message.starts_with("blocks:")
                    || message.starts_with("accounts:")
                    || message.starts_with("mining:")
                {
                    let identifier = message.split(':').next().unwrap();
                    let data = message
                        .strip_prefix(format!("{}:", identifier).as_str())
                        .unwrap_or("")
                        .to_string();
                    if data.is_empty() {
                        continue;
                    }

                    let mut ledger = state.ledger.lock().await;
                    let mut storage = state.storage.lock().await;

                    match receive_from_peer(data, &mut ledger, &mut storage, &identifier).await {
                        Ok(response) => {
                            if let Err(e) = safe_send(&state.sink, &response).await {
                                eprintln!(
                                    "Server: Failed to send block response to {}: {}",
                                    peer_addr, e
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Error processing block: {}", e);
                            if let Err(e) = safe_send(&state.sink, &error_msg).await {
                                eprintln!(
                                    "Server: Failed to send error response to {}: {}",
                                    peer_addr, e
                                );
                                break;
                            }
                        }
                    }
                } else if !message.starts_with("Echo:") {
                    let response = format!("Echo: {}", message);
                    if let Err(e) = safe_send(&state.sink, &response).await {
                        eprintln!("Server: Failed to send response to {}: {}", peer_addr, e);
                        break;
                    }
                    println!("Server: Sent response to {}", peer_addr);
                }
            }
            Ok(None) => {
                println!("Server: Connection closed by {}", peer_addr);
                break;
            }
            Err(e) => {
                eprintln!("Server: Error reading from {}: {}", peer_addr, e);
                break;
            }
        }
    }
    println!("Server: Closing connection with {}", peer_addr);
    Ok(())
}

async fn handle_client_connection(
    stream: TcpStream,
    state: Arc<SharedState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let peer_addr = stream.peer_addr()?;
    println!("Client: Connected to server at {}", peer_addr);

    let (reader, writer) = io::split(stream);
    let mut reader = BufReader::new(reader);
    let sink: Arc<dyn MessageSink> = Arc::new(TcpSink(Mutex::new(writer)));

    {
        let mut slot = state.sink.lock().await;
        *slot = Some(sink.clone());
    }

    sink.send("Hello from client!").await?;
    println!("Client: Sent greeting to server");

    let client_sender = tokio::spawn(async move {
        let mut count = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let message = format!("Client message #{}", count);
            if let Err(e) = sink.send(&message).await {
                eprintln!("Client: Failed to send message: {}", e);
                break;
            }
            println!("Client: Sent message: {}", message);
            count += 1;
        }
    });

    loop {
        match read_message(&mut reader).await {
            Ok(Some(message)) => {
                println!("Client received: {}", message);
            }
            Ok(None) => {
                println!("Client: Server closed connection");
                break;
            }
            Err(e) => {
                eprintln!("Client: Error reading from server: {}", e);
                break;
            }
        }
    }

    client_sender.abort();
    println!("Client: Closing connection with server");
    Ok(())
}

pub async fn start_network_listener(addr: &str, state: Arc<SharedState>) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Server: Listener started on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state_clone = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_server_connection(stream, state_clone).await {
                        eprintln!("Server: Error handling connection: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Server: Failed to accept connection: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn start_network_connector(addr: &str, state: Arc<SharedState>) -> io::Result<()> {
    match TcpStream::connect(addr).await {
        Ok(stream) => {
            println!("Client: Successfully connected to {}", addr);
            if let Err(e) = handle_client_connection(stream, state).await {
                eprintln!("Client: Error handling connection: {}", e);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Client: Failed to connect to {}: {}", addr, e);
            Err(e)
        }
    }
}
