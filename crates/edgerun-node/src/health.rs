use std::net::SocketAddr;

/// Shared health state updated by the daemon.
#[derive(Clone, Debug)]
pub struct HealthState {
    pub node_id: String,
    pub stream_id: String,
    pub started_at: std::time::Instant,
}

/// Runs a simple HTTP health server on the given port.
///
/// GET /health -> { "status": "ok", "uptime_secs": N, "node_id": "...", "stream_id": "..." }
pub async fn run_health_server(port: u16, state: HealthState) {
    use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match edgerun_rt::AsyncTcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => {
            edgerun_log::error!("failed to bind health endpoint on {}: {}", addr, e);
            return;
        }
    };
    edgerun_log::info!("health endpoint listening");

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let state = state.clone();
                edgerun_rt::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf).await;
                    let uptime = state.started_at.elapsed().as_secs();
                    let body = format!(
                        r#"{{"status":"ok","uptime_secs":{},"node_id":"{}","stream_id":"{}"}}"#,
                        uptime, state.node_id, state.stream_id
                    );
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.flush().await;
                });
            }
            Err(e) => {
                edgerun_log::warn!("health endpoint accept error: {}", e);
            }
        }
    }
}
