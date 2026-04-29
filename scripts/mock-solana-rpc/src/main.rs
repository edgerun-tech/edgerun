//! Mock Solana RPC server for testing.
//!
//! This provides a simple mock that responds to Solana RPC calls for testing
//! the edgerun-marketplace-cli without needing a real Solana cluster.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug)]
struct MockState {
    transactions: Mutex<Vec<String>>,
}

impl MockState {
    fn new() -> Self {
        Self {
            transactions: Mutex::new(Vec::new()),
        }
    }
}

fn make_provider_response() -> String {
    r#"{
        "jsonrpc": "2.0",
        "result": {
            "data": [
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                4,0,0,0,0,0,0,0,
                0,0,0,0,0,0,128,64,0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,32,64,0,0,0,0,0,0,0,0,
                100,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0,
                232,3,0,0,0,0,0,0,
                1,0,0,0
            ],
            "executable": false,
            "lamports": 1000000000,
            "owner": "EgRPRoGiVa7pBq7f9VTvJfJqLQKxVTuKpPQqM8dFLPer",
            "rentEpoch": 18446744073709551615,
            "space": 136
        },
        "id": 1
    }"#
    .to_string()
}

fn make_deployment_response() -> String {
    r#"{
        "jsonrpc": "2.0", 
        "result": {
            "data": [
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
                1,0,0,0,0,0,0,0
            ],
            "executable": false,
            "lamports": 1000000000,
            "owner": "DePLoYMtGaqDqLxU1vA3KqLQKxVTuKpPQqM8dFLPer",
            "rentEpoch": 18446744073709551615,
            "space": 200
        },
        "id": 1
    }"#
    .to_string()
}

fn make_empty_response() -> String {
    r#"{"jsonrpc": "2.0", "error": {"code": -32601, "message": "Method not found"}, "id": 1}"#
        .to_string()
}

fn make_send_response(tx_hash: &str) -> String {
    format!(r#"{{"jsonrpc": "2.0", "result": "{}", "id": 1}}"#, tx_hash)
}

fn make_signature_status_response() -> String {
    r#"{
        "jsonrpc": "2.0",
        "result": {
            "context": { "slot": 1 },
            "value": [
                {
                    "slot": 1,
                    "confirmations": 1,
                    "err": null,
                    "confirmationStatus": "confirmed"
                }
            ]
        },
        "id": 1
    }"#
    .to_string()
}

fn make_rent_response() -> String {
    r#"{"jsonrpc": "2.0", "result": 0, "id": 1}"#.to_string()
}

fn make_program_accounts_response() -> String {
    r#"{"jsonrpc": "2.0", "result": [], "id": 1}"#.to_string()
}

fn handle_request(state: &MockState, body: &str) -> String {
    // Simple JSON-RPC parsing
    if body.contains("getLatestBlockhash") {
        return r#"{
            "jsonrpc": "2.0",
            "result": {
                "context": { "slot": 1 },
                "value": {
                    "blockhash": "11111111111111111111111111111111",
                    "lastValidBlockHeight": 150
                }
            },
            "id": 1
        }"#
        .to_string();
    }

    if body.contains("getMinimumBalanceForRentExemption") {
        return make_rent_response();
    }

    if body.contains("getSignatureStatuses") {
        return make_signature_status_response();
    }

    if body.contains("getProgramAccounts") {
        return make_program_accounts_response();
    }

    if body.contains("getAccountInfo") {
        if body.contains("EgRPRoGiVa7pBq7f9VTvJfJqLQKxVTuKpPQqM8dFLPer") {
            return make_provider_response();
        } else if body.contains("DePLoYMtGaqDqLxU1vA3KqLQKxVTuKpPQqM8dFLPer") {
            return make_deployment_response();
        }
    }

    if body.contains("sendTransaction") {
        let tx_hash = "mocktx123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUV";
        state.transactions.lock().unwrap().push(tx_hash.to_string());
        return make_send_response(tx_hash);
    }

    make_empty_response()
}

async fn handle_connection(state: Arc<MockState>, mut stream: TcpStream) {
    let mut buf = vec![0u8; 8192];
    let n = match stream.read(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let mut request = buf[..n].to_vec();
    if let Some(content_length) = content_length(&request) {
        while request_body_len(&request) < content_length {
            let mut chunk = [0u8; 8192];
            let n = match stream.read(&mut chunk).await {
                Ok(n) if n > 0 => n,
                _ => break,
            };
            request.extend_from_slice(&chunk[..n]);
        }
    }

    let body = String::from_utf8_lossy(&request).to_string();

    let response = handle_request(&state, &body);

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        response.len(),
        response
    );

    stream.write_all(response.as_bytes()).await.ok();
}

fn content_length(request: &[u8]) -> Option<usize> {
    let headers = String::from_utf8_lossy(request);
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse().ok())
            .flatten()
    })
}

fn request_body_len(request: &[u8]) -> usize {
    request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|idx| request.len().saturating_sub(idx + 4))
        .unwrap_or(0)
}

#[tokio::main]
async fn main() {
    let addr: SocketAddr = std::env::var("MOCK_SOLANA_RPC_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8899".to_string())
        .parse()
        .unwrap();
    let listener = TcpListener::bind(addr).await.unwrap();

    let state = Arc::new(MockState::new());

    println!("Mock Solana RPC server running on http://{}", addr);
    println!("Press Ctrl+C to stop");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    handle_connection(state, stream).await;
                });
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }
}
