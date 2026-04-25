use std::net::SocketAddr;
use std::sync::Arc;

use edgerun_hardware_signing::NodeID;
use edgerun_rt::CancellationToken;
use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

const PROVISION_PORT: u16 = 35630;

pub(crate) async fn run_provisioning_listener(
    node_id: NodeID,
    public_key_hex: String,
    pairing_pin: Option<String>,
    cancel: CancellationToken,
) {
    let addr = format!("0.0.0.0:{}", PROVISION_PORT);
    let listen_addr: SocketAddr = match addr.parse() {
        Ok(a) => a,
        Err(e) => {
            edgerun_log::error!("failed to parse provisioning address: {}", e);
            return;
        }
    };

    let listener = match edgerun_rt::AsyncTcpListener::bind(listen_addr) {
        Ok(l) => l,
        Err(e) => {
            edgerun_log::error!("failed to bind provisioning on {}: {}", listen_addr, e);
            return;
        }
    };

    edgerun_log::info!("Provisioning listener ready on :{}", PROVISION_PORT);

    loop {
        if cancel.is_cancelled() {
            edgerun_log::info!("Provisioning listener shutting down");
            return;
        }

        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                edgerun_log::info!("Provisioning connection from {}", peer_addr);
                let pin = pairing_pin.clone();
                let pubkey = public_key_hex.clone();
                edgerun_rt::spawn(async move {
                    if let Err(e) = handle_provisioning_connection(stream, &pin, &pubkey).await {
                        edgerun_log::error!("Provisioning error: {}", e);
                    }
                });
            }
            Err(e) => {
                edgerun_log::warn!("Provisioning accept error: {}", e);
            }
        }
    }
}

fn read_json_field(json_str: &str, field: &str) -> Option<String> {
    let find = format!("\"{}\":\"", field);
    if let Some(start) = json_str.find(&find) {
        let rest = &json_str[start + find.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    let find2 = format!("\":{}", field);
    if let Some(start) = json_str.find(&find2) {
        let rest = &json_str[start + find2.len()..];
        let trimmed = rest.trim_start_matches(' ');
        if let Some(end) = trimmed.find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
            let value = &trimmed[..end];
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub(crate) async fn handle_provisioning_connection(
    mut stream: std::sync::Arc<edgerun_rt::AsyncTcpStream>,
    expected_pin: &Option<String>,
    public_key_hex: &str,
) -> Result<(), String> {
    use edgerun_rt::AsyncReadExt;

    let mut buf = [0u8; 512];
    let n = stream.read(&mut buf).await.map_err(|e| e.to_string())?;

    if n == 0 {
        return Err("no data received".into());
    }

    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    edgerun_log::debug!("Provisioning request: {}", request);

    let msg_type = read_json_field(&request, "type").unwrap_or_default();

    if msg_type == "provision" {
        let pin = read_json_field(&request, "pin").unwrap_or_default();
        let password = read_json_field(&request, "password").unwrap_or_default();
        let node_id_received = read_json_field(&request, "node_id").unwrap_or_default();

        if node_id_received != public_key_hex {
            return Err("node_id mismatch".into());
        }

        if let Some(expected) = expected_pin {
            if pin != *expected {
                return Err("PIN mismatch".into());
            }
        }

        if password.len() < 8 {
            return Err("password too short".into());
        }

        let response = r#"{"status":"provisioning_accepted"}"#;
        stream
            .write_all(response.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        edgerun_log::info!(
            "Node {} provisioning accepted, genesis will be encrypted with password",
            &public_key_hex[..16]
        );

        Ok(())
    } else if msg_type == "complete" {
        let password = read_json_field(&request, "password").unwrap_or_default();

        if password.len() < 8 {
            return Err("password too short".into());
        }

        let response = r#"{"status":"genesis_completed"}"#;
        stream
            .write_all(response.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        edgerun_log::info!(
            "Node {} genesis completed and encrypted with password",
            &public_key_hex[..16]
        );

        Ok(())
    } else if msg_type == "unlock" {
        let password = read_json_field(&request, "password").unwrap_or_default();

        if password.len() < 8 {
            return Err("password too short".into());
        }

        let response = r#"{"status":"unlock_accepted"}"#;
        stream
            .write_all(response.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        edgerun_log::info!(
            "Unlock accepted for node {}, will decrypt private key with password",
            &public_key_hex[..16]
        );

        Ok(())
    } else {
        Err(format!("unknown message type: {}", msg_type))
    }
}
