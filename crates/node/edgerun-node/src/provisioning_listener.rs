use std::fs;
use std::path::Path;
use std::sync::Arc;

use edgerun_hardware_signing::NodeID;
use edgerun_json::{escape_json_string, Value as JsonValue};
use edgerun_rt::CancellationToken;
use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

const PROVISION_PORT: u16 = 35630;

pub(crate) async fn run_provisioning_listener(
    _node_id: NodeID,
    public_key_hex: String,
    pairing_pin: Option<String>,
    config_path: Option<std::path::PathBuf>,
    cancel: CancellationToken,
) {
    let addr = format!("0.0.0.0:{PROVISION_PORT}");
    let listen_addr: std::net::SocketAddr = match addr.parse() {
        Ok(a) => a,
        Err(e) => {
            edgerun_log::error!("failed to parse provisioning address: {e}");
            return;
        }
    };

    let listener = match edgerun_rt::AsyncTcpListener::bind(listen_addr) {
        Ok(l) => l,
        Err(e) => {
            edgerun_log::error!("failed to bind provisioning on {listen_addr}: {e}");
            return;
        }
    };

    edgerun_log::info!("Provisioning listener ready on :{PROVISION_PORT}");
    if let Some(path) = &config_path {
        edgerun_log::info!("Provisioning persistence enabled via {}", path.display());
    }

    loop {
        if cancel.is_cancelled() {
            edgerun_log::info!("Provisioning listener shutting down");
            return;
        }

        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                edgerun_log::info!("Provisioning connection from {peer_addr}");
                let pin = pairing_pin.clone();
                let pubkey = public_key_hex.clone();
                let target_config_path = config_path.clone();
                let peer_loopback = peer_addr.ip().is_loopback();
                edgerun_rt::spawn(async move {
                    if let Err(e) = handle_provisioning_connection(
                        stream,
                        &pin,
                        &pubkey,
                        target_config_path.as_deref(),
                        peer_loopback,
                    )
                    .await
                    {
                        edgerun_log::error!("Provisioning error: {e}");
                    }
                });
            }
            Err(e) => {
                edgerun_log::warn!("Provisioning accept error: {e}");
            }
        }
    }
}

fn json_text(value: Option<&JsonValue>, _key: &str) -> Option<String> {
    value
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string)
}

fn response_ok(status: &str) -> String {
    format!(r#"{{"status":"{}"}}"#, escape_json_string(status))
}

fn response_err(error: &str) -> String {
    format!(
        r#"{{"status":"error","error":"{}"}}"#,
        escape_json_string(error)
    )
}

pub(crate) async fn handle_provisioning_connection(
    mut stream: Arc<edgerun_rt::AsyncTcpStream>,
    expected_pin: &Option<String>,
    public_key_hex: &str,
    config_path: Option<&Path>,
    peer_loopback: bool,
) -> Result<(), String> {
    let mut buf = [0u8; 512];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|error| error.to_string())?;
    if n == 0 {
        return Err("no data received".into());
    }

    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    edgerun_log::debug!("Provisioning request: {request}");

    let payload: JsonValue = match edgerun_json::from_json_slice(request.as_bytes()) {
        Ok(payload) => payload,
        Err(error) => {
            let body = response_err(&format!("invalid JSON: {error}"));
            stream
                .write_all(body.as_bytes())
                .await
                .map_err(|error| error.to_string())?;
            return Err("invalid JSON payload".into());
        }
    };

    let msg_type = json_text(payload.get("type"), "type").unwrap_or_default();
    let pin = json_text(payload.get("pin"), "pin").unwrap_or_default();
    let node_id = json_text(payload.get("node_id"), "node_id").unwrap_or_default();
    let skip_local_checks = payload
        .get("skip_checks")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let bypass_security = skip_local_checks && peer_loopback;

    let response = if msg_type == "provision" {
        if node_id != public_key_hex {
            response_err("node_id mismatch")
        } else if !bypass_security {
            if let Some(expected) = expected_pin {
                if pin != *expected {
                    response_err("PIN mismatch")
                } else {
                    persist_provisioning_response(config_path, public_key_hex)
                }
            } else {
                persist_provisioning_response(config_path, public_key_hex)
            }
        } else {
            persist_provisioning_response(config_path, public_key_hex)
        }
    } else if msg_type == "complete" {
        response_ok("genesis_completed")
    } else {
        response_err(&format!("unknown message type: {msg_type}"))
    };

    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn persist_provisioning_response(config_path: Option<&Path>, public_key_hex: &str) -> String {
    if let Some(path) = config_path {
        if let Err(error) = persist_provisioned_signer_state(path, public_key_hex) {
            return response_err(&format!("failed to persist provisioning state: {error}"));
        }
    }
    response_ok("provisioning_accepted")
}

fn persist_provisioned_signer_state(
    config_path: &Path,
    public_key_hex: &str,
) -> Result<(), String> {
    let raw = fs::read_to_string(config_path).map_err(|error| error.to_string())?;
    let lines = raw.split_inclusive('\n').collect::<Vec<_>>();
    let mut signer_start = None;
    let mut signer_end = None;
    let mut cursor = 0;
    while cursor < lines.len() {
        let line = lines[cursor];
        if line.trim_end_matches('\n') != "signer:" || line.starts_with(' ') {
            cursor += 1;
            continue;
        }

        let mut end = cursor + 1;
        while end < lines.len() && lines[end].starts_with("  ") {
            end += 1;
        }

        let block = &lines[cursor + 1..end];
        let mut signer_type: Option<String> = None;
        let mut is_target_signer = false;
        for entry in block {
            if let Some((key, value)) = crate::config::parse_kv(entry.trim_start()) {
                let value = crate::config::unquote(&value.trim_end_matches('\n'));
                if key == "type" {
                    signer_type = Some(value.to_string());
                } else if key == "public_key_hex" && value == public_key_hex {
                    is_target_signer = true;
                }
            }
        }

        if signer_type.as_deref() == Some("provisioned") && is_target_signer {
            signer_start = Some(cursor);
            signer_end = Some(end);
            break;
        }
        cursor = end;
    }

    let (start, end) = match (signer_start, signer_end) {
        (Some(start), Some(end)) => (start, end),
        _ => return Err("target provisioned signer not found".into()),
    };

    let block = &lines[start + 1..end];
    let mut has_state = false;

    let mut output = String::new();
    for line in lines[..start + 1].iter() {
        output.push_str(line);
    }

    for line in block {
        let raw_line = line.trim_end_matches('\n');
        if let Some((key, value)) = crate::config::parse_kv(raw_line.trim_start()) {
            if key == "state" {
                output.push_str(&format!("  state: \"active\"\n"));
                has_state = true;
                continue;
            }
            if key == "pairing_pin" {
                continue;
            }
            output.push_str(line);
            continue;
        }
        output.push_str(line);
    }

    if !has_state {
        output.push_str("  state: \"active\"\n");
    }

    for line in lines[end..].iter() {
        output.push_str(line);
    }

    fs::write(config_path, output).map_err(|error| error.to_string())?;
    edgerun_log::info!(
        "provisioning state persisted for node {}",
        &public_key_hex[..16]
    );
    Ok(())
}
