use std::path::Path;
use std::sync::Arc;

use edgerun_hardware_signing::NodeID;
use edgerun_json::{Value as JsonValue, escape_json_string};
use edgerun_node::network::{HostSocketTransport, TransportAddress};
use edgerun_node::rt::{AsyncReadExt, AsyncWriteExt};
use edgerun_node::rt::{CancellationToken, timeout};

const PROVISION_PORT: u16 = 35630;
const ACCEPT_POLL_INTERVAL: core::time::Duration = core::time::Duration::from_millis(100);

pub(crate) async fn run_provisioning_listener(
    _node_id: NodeID,
    public_key_hex: String,
    pairing_pin: Option<String>,
    config_path: Option<std::path::PathBuf>,
    cancel: CancellationToken,
) {
    let addr = format!("127.0.0.1:{PROVISION_PORT}");
    let listen_addr: std::net::SocketAddr = match addr.parse() {
        Ok(a) => a,
        Err(e) => {
            crate::node_error!("failed to parse provisioning address: {e}");
            return;
        }
    };

    let listener = match HostSocketTransport.bind_stream_now(&TransportAddress::host_stream(
        listen_addr.to_string().into_bytes(),
    )) {
        Ok(l) => l,
        Err(e) => {
            crate::node_error!("failed to bind provisioning on {listen_addr}: {e}");
            return;
        }
    };

    crate::node_info!("Provisioning listener ready on 127.0.0.1:{PROVISION_PORT}");
    if let Some(path) = &config_path {
        crate::node_info!("Provisioning persistence enabled via {}", path.display());
    }

    loop {
        if cancel.is_cancelled() {
            crate::node_info!("Provisioning listener shutting down");
            return;
        }

        match timeout(ACCEPT_POLL_INTERVAL, listener.accept()).await {
            Err(_) => continue,
            Ok(Ok((stream, peer_addr))) => {
                crate::node_info!("Provisioning connection from {peer_addr}");
                let pin = pairing_pin.clone();
                let pubkey = public_key_hex.clone();
                let target_config_path = config_path.clone();
                edgerun_node::rt::spawn(async move {
                    if let Err(e) = handle_provisioning_connection(
                        stream,
                        &pin,
                        &pubkey,
                        target_config_path.as_deref(),
                    )
                    .await
                    {
                        crate::node_error!("Provisioning error: {e}");
                    }
                });
            }
            Ok(Err(e)) => {
                crate::node_warn!("Provisioning accept error: {e}");
                edgerun_node::rt::sleep(ACCEPT_POLL_INTERVAL).await;
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
    mut stream: Arc<edgerun_node::rt::AsyncTcpStream>,
    expected_pin: &Option<String>,
    public_key_hex: &str,
    config_path: Option<&Path>,
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
    crate::node_debug!("Provisioning request received: {} bytes", n);

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

    let response = if msg_type == "provision" {
        if node_id != public_key_hex {
            response_err("node_id mismatch")
        } else {
            if let Some(expected) = expected_pin {
                if pin != *expected {
                    response_err("PIN mismatch")
                } else {
                    persist_provisioning_response(config_path, public_key_hex)
                }
            } else {
                persist_provisioning_response(config_path, public_key_hex)
            }
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
    _config_path: &Path,
    public_key_hex: &str,
) -> Result<(), String> {
    crate::node_info!(
        "provisioning accepted for node {}; authoritative changes must be committed to the event log",
        &public_key_hex[..16]
    );
    Ok(())
}
