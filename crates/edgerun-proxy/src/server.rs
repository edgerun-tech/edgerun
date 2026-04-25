use edgerun_log::{debug, info, warn};
use edgerun_rt::{
    spawn, timeout, AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncWriteExt,
    CancellationToken, TcpSocket,
};
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ProxyConfig {
    pub bind_addr: String,
    pub socks5_bind_addr: Option<String>,
    pub upstream_proxy: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub connect_timeout: Duration,
    pub tunnel_buffer_size: usize,
    pub tunnel_read_timeout: Duration,
    pub tunnel_write_timeout: Duration,
    pub max_connections: Option<usize>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8080".to_string(),
            socks5_bind_addr: None,
            upstream_proxy: None,
            username: None,
            password: None,
            connect_timeout: Duration::from_secs(30),
            tunnel_buffer_size: 64 * 1024,
            tunnel_read_timeout: Duration::from_secs(60),
            tunnel_write_timeout: Duration::from_secs(60),
            max_connections: None,
        }
    }
}

pub struct ProxyServer {
    config: ProxyConfig,
    active_connections: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    graceful_shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ProxyServer {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            active_connections: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            graceful_shutdown: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    #[allow(dead_code)]
    fn is_shutting_down(&self) -> bool {
        self.graceful_shutdown.load(Ordering::Relaxed)
    }

    pub async fn run(&self, shutdown: CancellationToken) -> std::io::Result<()> {
        let http_listener = AsyncTcpListener::bind(&self.config.bind_addr)?;
        info!("HTTP proxy listening on {}", self.config.bind_addr);

        let socks5_listener = if let Some(ref addr) = self.config.socks5_bind_addr {
            let listener = AsyncTcpListener::bind(addr)?;
            info!("SOCKS5 proxy listening on {}", addr);
            Some(listener)
        } else {
            None
        };

        let http_handle = spawn({
            let config = self.config.clone();
            let listener = http_listener;
            let active = self.active_connections.clone();
            let max = self.config.max_connections;
            let graceful = self.graceful_shutdown.clone();
            async move {
                loop {
                    if graceful.load(Ordering::Relaxed) {
                        debug!("Shutdown initiated, stopping HTTP accept loop");
                        break;
                    }
                    match listener.accept().await {
                        Ok((socket, addr)) => {
                            if let Some(limit) = max {
                                if active.load(Ordering::Relaxed) >= limit {
                                    debug!("Connection limit reached, rejecting {}", addr);
                                    continue;
                                }
                            }
                            active.fetch_add(1, Ordering::Relaxed);
                            let config = config.clone();
                            let active = active.clone();
                            let _graceful = graceful.clone();
                            spawn(async move {
                                let result = handle_http_proxy(socket, addr, config).await;
                                active.fetch_sub(1, Ordering::Relaxed);
                                if let Err(e) = result {
                                    debug!("HTTP proxy error from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => warn!("HTTP accept error: {}", e),
                    }
                }
            }
        });

        let socks5_handle = if let Some(listener) = socks5_listener {
            let config = self.config.clone();
            let active = self.active_connections.clone();
            let max = self.config.max_connections;
            let graceful = self.graceful_shutdown.clone();
            Some(spawn(async move {
                loop {
                    if graceful.load(Ordering::Relaxed) {
                        debug!("Shutdown initiated, stopping SOCKS5 accept loop");
                        break;
                    }
                    match listener.accept().await {
                        Ok((socket, addr)) => {
                            if let Some(limit) = max {
                                if active.load(Ordering::Relaxed) >= limit {
                                    debug!("Connection limit reached, rejecting {}", addr);
                                    continue;
                                }
                            }
                            active.fetch_add(1, Ordering::Relaxed);
                            let config = config.clone();
                            let active = active.clone();
                            spawn(async move {
                                let result = handle_socks5(socket, addr, config).await;
                                active.fetch_sub(1, Ordering::Relaxed);
                                if let Err(e) = result {
                                    debug!("SOCKS5 error from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => warn!("SOCKS5 accept error: {}", e),
                    }
                }
            }))
        } else {
            None
        };

        shutdown.cancelled().await;

        self.graceful_shutdown.store(true, Ordering::Relaxed);
        while self.active_connections.load(Ordering::Relaxed) > 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let _ = http_handle.await;
        if let Some(h) = socks5_handle {
            let _ = h.await;
        }

        info!("Proxy server shutting down");
        Ok(())
    }
}

const SOCKS5_VERSION: u8 = 0x05;
const SOCKS5_CMD_CONNECT: u8 = 0x01;
const SOCKS5_ATYP_IPV4: u8 = 0x01;
const SOCKS5_ATYP_DOMAIN: u8 = 0x03;
const SOCKS5_ATYP_IPV6: u8 = 0x04;
const SOCKS5_REP_SUCCESS: u8 = 0x00;
const SOCKS5_REP_GENERAL_FAILURE: u8 = 0x01;
const SOCKS5_REP_CONN_REFUSED: u8 = 0x05;
const SOCKS5_REP_TTL_EXPIRED: u8 = 0x06;
const SOCKS5_REP_CMD_NOT_SUPPORTED: u8 = 0x07;
const SOCKS5_REP_ADDR_NOT_SUPPORTED: u8 = 0x08;

async fn handle_http_proxy(
    socket: Arc<AsyncTcpStream>,
    _addr: SocketAddr,
    config: ProxyConfig,
) -> std::io::Result<()> {
    let mut socket = socket.clone();
    let mut buffer = vec![0u8; config.tunnel_buffer_size];

    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    let first_line = match request.lines().next() {
        Some(l) => l,
        None => return write_response(&mut socket, 400, "Bad Request").await,
    };

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return write_response(&mut socket, 400, "Bad Request").await;
    }

    match parts[0] {
        "CONNECT" => handle_connect_tunnel(socket, parts[1], &config).await,
        _ => {
            if let Some(ref upstream) = config.upstream_proxy {
                let (host, port) = parse_upstream_proxy(upstream);
                handle_http_forward(socket, parts, &request, host, port).await
            } else {
                write_response(&mut socket, 400, "Use CONNECT for tunneling").await
            }
        }
    }
}

async fn handle_connect_tunnel(
    socket: Arc<AsyncTcpStream>,
    target: &str,
    config: &ProxyConfig,
) -> std::io::Result<()> {
    let (host, port) = parse_target(target);

    let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();

    let is_ipv6 = host.starts_with('[') || host.contains(':');
    let sock = if is_ipv6 {
        TcpSocket::new_v6()?
    } else {
        TcpSocket::new_v4()?
    };

    let connect_result = timeout(config.connect_timeout, sock.connect(addr)).await;

    match connect_result {
        Ok(Ok(upstream)) => {
            let mut socket = socket;
            socket
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .await?;
            tunnel_bidirectional(socket, upstream).await;
            Ok(())
        }
        Ok(Err(e)) => {
            let mut socket = socket;
            write_response(&mut socket, 502, &format!("Connection failed: {}", e)).await
        }
        Err(_) => {
            let mut socket = socket;
            write_response(&mut socket, 504, "Gateway Timeout").await
        }
    }
}

async fn tunnel_bidirectional(a: Arc<AsyncTcpStream>, b: Arc<AsyncTcpStream>) {
    use edgerun_rt::copy_bidirectional;

    let mut a = a;
    let mut b = b;
    let _ = copy_bidirectional(&mut a, &mut b).await;
}

fn parse_upstream_proxy(proxy: &str) -> (&str, u16) {
    let parts: Vec<&str> = proxy.split(':').collect();
    let host = *parts.first().unwrap_or(&"localhost");
    let port = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(8080);
    (host, port)
}

fn parse_target(target: &str) -> (&str, u16) {
    let parts: Vec<&str> = target.split(':').collect();
    let host = if parts.len() > 1 && parts[0].is_empty() {
        &parts[0][1..]
    } else {
        *parts.first().unwrap_or(&target)
    };
    let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(443);
    (host, port)
}

async fn handle_http_forward(
    mut socket: Arc<AsyncTcpStream>,
    parts: Vec<&str>,
    request: &str,
    upstream_host: &str,
    upstream_port: u16,
) -> std::io::Result<()> {
    let addr: SocketAddr = format!("{}:{}", upstream_host, upstream_port)
        .parse()
        .unwrap();

    let sock = TcpSocket::new_v4()?;
    let upstream = match timeout(std::time::Duration::from_secs(30), sock.connect(addr)).await {
        Ok(Ok(s)) => s,
        _ => return write_response(&mut socket, 502, "Upstream unreachable").await,
    };

    let method = parts[0];
    let path = parts[1];

    let lines: Vec<&str> = request.lines().collect();
    let host_idx = lines
        .iter()
        .position(|l| l.to_lowercase().starts_with("host:"));
    let host_header = if let Some(idx) = host_idx {
        lines[idx]
            .strip_prefix("Host:")
            .map(|s| s.trim())
            .unwrap_or("")
    } else {
        ""
    };

    let forwarded = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        method, path, host_header
    );

    socket.write_all(forwarded.as_bytes()).await?;
    tunnel_bidirectional(socket, upstream).await;
    Ok(())
}

async fn handle_socks5(
    socket: Arc<AsyncTcpStream>,
    _addr: SocketAddr,
    config: ProxyConfig,
) -> std::io::Result<()> {
    let mut socket = socket.clone();
    let mut buffer = vec![0u8; 1024];

    let n = socket.read(&mut buffer).await?;
    if n < 3 || buffer[0] != SOCKS5_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "only SOCKS5 supported",
        ));
    }

    let method_count = buffer[1] as usize;
    let methods = buffer.get(2..2 + method_count).unwrap_or(&[]);
    let has_no_auth = methods.contains(&0x00);
    let has_userpass = methods.contains(&0x02);

    let method = if has_userpass {
        0x02
    } else if has_no_auth {
        0x00
    } else {
        0xFF
    };

    if method == 0xFF {
        socket.write_all(&[SOCKS5_VERSION, 0xFF]).await?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "no acceptable auth",
        ));
    }

    socket.write_all(&[SOCKS5_VERSION, method]).await?;

    if method == 0x02 {
        authenticate_socks5(&mut socket, &config).await?
    }

    let n = socket.read(&mut buffer).await?;
    if n < 5 || buffer[0] != SOCKS5_VERSION || buffer[2] != 0x00 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "bad request",
        ));
    }

    let cmd = buffer[1];
    if cmd != SOCKS5_CMD_CONNECT {
        send_socks_reply(
            &mut socket,
            SOCKS5_REP_CMD_NOT_SUPPORTED,
            SOCKS5_ATYP_IPV4,
            "",
            0,
        )
        .await?;
        return Ok(());
    }

    let (host, port) = match buffer[3] {
        SOCKS5_ATYP_IPV4 if n >= 10 => {
            let ip = format!("{}.{}.{}.{}", buffer[4], buffer[5], buffer[6], buffer[7]);
            let port = u16::from_be_bytes([buffer[8], buffer[9]]);
            (ip, port)
        }
        SOCKS5_ATYP_IPV6 if n >= 22 => {
            let ip = format!(
                "{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}",
                buffer[4], buffer[5], buffer[6], buffer[7], buffer[8], buffer[9], buffer[10], buffer[11],
                buffer[12], buffer[13], buffer[14], buffer[15], buffer[16], buffer[17], buffer[18], buffer[19]
            );
            let port = u16::from_be_bytes([buffer[20], buffer[21]]);
            (ip, port)
        }
        SOCKS5_ATYP_DOMAIN => {
            let len = buffer[4] as usize;
            if n < 5 + len + 2 {
                send_socks_reply(
                    &mut socket,
                    SOCKS5_REP_ADDR_NOT_SUPPORTED,
                    SOCKS5_ATYP_IPV4,
                    "",
                    0,
                )
                .await?;
                return Ok(());
            }
            let host = String::from_utf8_lossy(&buffer[5..5 + len]).to_string();
            let port = u16::from_be_bytes([buffer[5 + len], buffer[6 + len]]);
            (host, port)
        }
        _ => {
            send_socks_reply(
                &mut socket,
                SOCKS5_REP_ADDR_NOT_SUPPORTED,
                SOCKS5_ATYP_IPV4,
                "",
                0,
            )
            .await?;
            return Ok(());
        }
    };

    let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
    let socket2 = TcpSocket::new_v4()?;
    let connect_result = timeout(config.connect_timeout, socket2.connect(addr)).await;

    match connect_result {
        Ok(Ok(upstream)) => {
            send_socks_reply(&mut socket, SOCKS5_REP_SUCCESS, SOCKS5_ATYP_IPV4, "", 0).await?;
            tunnel_bidirectional(upstream, socket).await;
            Ok(())
        }
        Ok(Err(e)) => {
            let rep = match e.kind() {
                std::io::ErrorKind::ConnectionRefused => SOCKS5_REP_CONN_REFUSED,
                std::io::ErrorKind::TimedOut => SOCKS5_REP_TTL_EXPIRED,
                _ => SOCKS5_REP_GENERAL_FAILURE,
            };
            send_socks_reply(&mut socket, rep, SOCKS5_ATYP_IPV4, "", 0).await
        }
        Err(_) => {
            send_socks_reply(&mut socket, SOCKS5_REP_TTL_EXPIRED, SOCKS5_ATYP_IPV4, "", 0).await
        }
    }
}

async fn authenticate_socks5(
    socket: &mut Arc<AsyncTcpStream>,
    config: &ProxyConfig,
) -> std::io::Result<()> {
    let mut buffer = vec![0u8; 515];
    let n = socket.read(&mut buffer).await?;
    if n < 3 || buffer[0] != 0x01 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid auth version",
        ));
    }

    let user_len = buffer[1] as usize;
    if n < 2 + user_len + 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "truncated credentials",
        ));
    }
    let pass_len = buffer[2 + user_len] as usize;
    if n < 2 + user_len + 1 + pass_len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "truncated credentials",
        ));
    }

    let username = String::from_utf8_lossy(&buffer[2..2 + user_len]).to_string();
    let password =
        String::from_utf8_lossy(&buffer[2 + user_len + 1..2 + user_len + 1 + pass_len]).to_string();

    let valid = match (&config.username, &config.password) {
        (Some(u), Some(p)) => u == &username && p == &password,
        _ => false,
    };

    if valid {
        socket.write_all(&[0x01, 0x00]).await?;
        Ok(())
    } else {
        socket.write_all(&[0x01, 0x01]).await?;
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "invalid credentials",
        ))
    }
}

async fn send_socks_reply(
    socket: &mut Arc<AsyncTcpStream>,
    rep: u8,
    atyp: u8,
    host: &str,
    port: u16,
) -> std::io::Result<()> {
    let mut reply = vec![SOCKS5_VERSION, rep, 0x00, atyp];
    match atyp {
        SOCKS5_ATYP_IPV4 => {
            reply.extend_from_slice(&[0, 0, 0, 0]);
            reply.extend_from_slice(&port.to_be_bytes());
        }
        SOCKS5_ATYP_DOMAIN => {
            reply.push(host.len() as u8);
            reply.extend_from_slice(host.as_bytes());
            reply.extend_from_slice(&port.to_be_bytes());
        }
        _ => {}
    }
    socket.write_all(&reply).await
}

async fn write_response(
    socket: &mut Arc<AsyncTcpStream>,
    status: u16,
    message: &str,
) -> std::io::Result<()> {
    let body = message.to_string();
    let status_text = match status {
        400 => "Bad Request",
        502 => "Bad Gateway",
        504 => "Gateway Timeout",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await
}
