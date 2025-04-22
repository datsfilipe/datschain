use std::net::SocketAddr;

pub fn get_listen_addr() -> String {
    std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string())
}

pub fn get_api_addr() -> SocketAddr {
    std::env::var("API_ADDR")
        .ok()
        .and_then(|addr_str| addr_str.parse::<SocketAddr>().ok())
        .unwrap_or_else(|| {
            eprintln!("Invalid or unset API_ADDR, defaulting to 127.0.0.1:3001");
            "127.0.0.1:3001"
                .parse()
                .expect("Default address is invalid")
        })
}

pub fn get_database_path() -> String {
    std::env::var("DATABASE_PATH").unwrap_or_else(|_| "/tmp/ledger".to_string())
}

pub fn get_peer_addresses() -> Vec<String> {
    match std::env::var("PEER_ADDRESSES") {
        Ok(addrs) => addrs
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}
