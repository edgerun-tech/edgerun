use edgerun_rt::{spawn, CancellationToken, timeout};
use edgerun_log::{info, debug, warn};
use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct ProxyConfig {
    pub bind_addr: String,
    pub socks5_bind_addr: Option<String>,
    pub upstream_proxy: Option<String>,
    pub connect_timeout: Duration,
    pub tunnel_buffer_size: usize,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8080".to_string(),
            socks5_bind_addr: None,
            upstream_proxy: None,
            connect_timeout: Duration::from_secs(30),
            tunnel_buffer_size: 64 * 1024,
        }
    }
}

pub struct ProxyServer {
    config: ProxyConfig,
}

impl ProxyServer {
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self, shutdown: CancellationToken) -> std::io::Result<()> {
        let http_listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("HTTP proxy listening on {}", self.config.bind_addr);

        let socks5_listener = if let Some(ref addr) = self.config.socks5_bind_addr {
            let listener = TcpListener::bind(addr).await?;
            info!("SOCKS5 proxy listening on {}", addr);
            Some(listener)
        } else {
            None
        };

        let config = self.config.clone();
        let http_task = spawn(async move {
            http_loop(http_listener, &config).await;
        });

        let socks5_task = if let Some(listener) = socks5_listener {
            let config = self.config.clone();
            Some(spawn(async move {
                socks5_loop(listener, &config).await;
            }))
        } else {
            None
        };

        shutdown.cancelled().await;

        info!("HTTP proxy shutting down");
        Ok(())
    }
}

async fn http_loop(listener: TcpListener, config: &ProxyConfig) {
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                let config = config.clone();
                spawn(async move {
                    if let Err(e) = handle_http_proxy(socket, addr, config).await {
                        debug!("HTTP proxy error from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                warn!("HTTP accept error: {}", e);
            }
        }
    }
}

async fn socks5_loop(listener: TcpListener, config: &ProxyConfig) {
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                let config = config.clone();
                spawn(async move {
                    if let Err(e) = handle_socks5(socket, addr, config).await {
                        debug!("SOCKS5 error from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                warn!("SOCKS5 accept error: {}", e);
            }
        }
    }
}

const SOCKS5_VERSION: u8 = 0x05;

const SOCKS5_CMD_CONNECT: u8 = 0x01;
const SOCKS5_CMD_BIND: u8 = 0x02;
const SOCKS5_CMD_UDP_ASSOCIATE: u8 = 0x03;

const SOCKS5_ATYP_IPV4: u8 = 0x01;
const SOCKS5_ATYP_DOMAIN: u8 = 0x03;
const SOCKS5_ATYP_IPV6: u8 = 0x04;

const SOCKS5_REP_SUCCESS: u8 = 0x00;
const SOCKS5_REP_GENERAL_FAILURE: u8 = 0x01;
const SOCKS5_REP_NOT_ALLOWED: u8 = 0x02;
const SOCKS5_REP_NET_UNREACHABLE: u8 = 0x03;
const SOCKS5_REP_HOST_UNREACHABLE: u8 = 0x04;
const SOCKS5_REP_CONN_REFUSED: u8 = 0x05;
const SOCKS5_REP_TTL_EXPIRED: u8 = 0x06;
const SOCKS5_REP_CMD_NOT_SUPPORTED: u8 = 0x07;
const SOCKS5_REP_ADDR_NOT_SUPPORTED: u8 = 0x08;

#[derive(Debug, Clone)]
struct Socks5Address {
    host: String,
    port: u16,
}

impl Socks5Address {
    fn parse(buffer: &[u8]) -> Option<Self> {
        if buffer.len() < 4 {
            return None;
        }

        match buffer[3] {
            SOCKS5_ATYP_IPV4 => {
                if buffer.len() < 10 {
                    return None;
                }
                let ip = IpAddr::from([buffer[4], buffer[5], buffer[6], buffer[7]]);
                let port = u16::from_be_bytes([buffer[8], buffer[9]]);
                Some(Socks5Address {
                    host: ip.to_string(),
                    port,
                })
            }
            SOCKS5_ATYP_DOMAIN => {
                let len = buffer[4] as usize;
                if buffer.len() < 5 + len + 2 {
                    return None;
                }
                let host = String::from_utf8_lossy(&buffer[5..5 + len]).to_string();
                let port = u16::from_be_bytes([buffer[5 + len], buffer[6 + len]]);
                Some(Socks5Address { host, port })
            }
            SOCKS5_ATYP_IPV6 => {
                if buffer.len() < 22 {
                    return None;
                }
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&buffer[4..20]);
                let ip = IpAddr::from(octets);
                let port = u16::from_be_bytes([buffer[20], buffer[21]]);
                Some(Socks5Address {
                    host: ip.to_string(),
                    port,
                })
            }
            _ => None,
        }
    }
}

async fn handle_http_proxy(socket: TcpStream, _addr: SocketAddr, config: ProxyConfig) -> std::io::Result<()> {
    let mut socket = socket;
    let mut buffer = vec![0u8; config.tunnel_buffer_size];

    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    let first_line = request.lines().next().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "no request line"))?;

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return write_response(&mut socket, 400, "Bad Request").await;
    }

    match parts[0] {
        "CONNECT" => {
            let uri = parts[1];
            handle_connect_tunnel(&mut socket, uri, &config).await
        }
        _ => {
            if config.upstream_proxy.is_some() {
                handle_upstream_proxy(&mut socket, &buffer[..n], &config).await
            } else {
                write_response(&mut socket, 400, "Use CONNECT for tunneling").await
            }
        }
    }
}

async fn handle_upstream_proxy(
    _socket: &mut TcpStream,
    _request: &[u8],
    _config: &ProxyConfig,
) -> std::io::Result<()> {
    write_response(_socket, 501, "Upstream proxy not implemented").await
}

async fn handle_connect_tunnel(socket: &mut TcpStream, target: &str, config: &ProxyConfig) -> std::io::Result<()> {
    let parts: Vec<&str> = target.split(':').collect();
    let host = parts.first().unwrap_or(&target);
    let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(443);

    let connect_result = timeout(
        config.connect_timeout,
        TcpStream::connect(format!("{}:{}", host, port)),
    )
    .await;

    match connect_result {
        Ok(Ok(upstream)) => {
            let response = b"HTTP/1.1 200 Connection Established\r\n\r\n";
            socket.write_all(response).await?;
            let mut socket = socket;
            tunnel_copy(&mut socket, upstream, config.tunnel_buffer_size).await?;
        }
        Ok(Err(e)) => {
            let msg = format!("502 Bad Gateway\r\nConnection failed: {}", e);
            write_response(socket, 502, &msg).await?;
        }
        Err(_) => {
            let msg = "504 Gateway Timeout\r\nConnection timed out";
            write_response(socket, 504, msg).await?;
        }
    }

    Ok(())
}

async fn handle_socks5(socket: TcpStream, _addr: SocketAddr, config: ProxyConfig) -> std::io::Result<()> {
    let mut socket = socket;
    let mut buffer = vec![0u8; 1024];

    let n = socket.read(&mut buffer).await?;
    if n < 3 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "handshake too short"));
    }

    if buffer[0] != SOCKS5_VERSION {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "only SOCKS5 supported"));
    }

    let method_count = buffer[1] as usize;
    if n < 2 + method_count {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "methods truncated"));
    }

    let has_no_auth = buffer[2..2 + method_count].contains(&0x00);
    if !has_no_auth {
        socket.write_all(&[SOCKS5_VERSION, 0xFF]).await?;
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no acceptable auth"));
    }

    socket.write_all(&[SOCKS5_VERSION, 0x00]).await?;

    let n = socket.read(&mut buffer).await?;
    if n < 5 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "request too short"));
    }

    if buffer[0] != SOCKS5_VERSION || buffer[2] != 0x00 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid request format"));
    }

    let cmd = buffer[1];
    let addr = match Socks5Address::parse(&buffer[..n]) {
        Some(a) => a,
        None => {
            send_socks_reply(&mut socket, SOCKS5_REP_ADDR_NOT_SUPPORTED, SOCKS5_ATYP_IPV4, "", 0).await?;
            return Ok(());
        }
    };

    match cmd {
        SOCKS5_CMD_CONNECT => {
            handle_socks5_connect(&mut socket, addr, &config).await;
        }
        SOCKS5_CMD_BIND | SOCKS5_CMD_UDP_ASSOCIATE => {
            send_socks_reply(&mut socket, SOCKS5_REP_CMD_NOT_SUPPORTED, SOCKS5_ATYP_IPV4, "", 0).await?;
        }
        _ => {
            send_socks_reply(&mut socket, SOCKS5_REP_GENERAL_FAILURE, SOCKS5_ATYP_IPV4, "", 0).await?;
        }
    }

    Ok(())
}

async fn handle_socks5_connect(socket: &mut TcpStream, addr: Socks5Address, config: &ProxyConfig) {
    let connect_result = timeout(
        config.connect_timeout,
        TcpStream::connect(format!("{}:{}", addr.host, addr.port)),
    )
    .await;

    match connect_result {
        Ok(Ok(upstream)) => {
            let local_addr = match upstream.local_addr() {
                Ok(a) => a,
                Err(_) => SocketAddr::from(([0, 0, 0, 0], 0)),
            };

            if let Err(e) = send_socks5_reply_connected(socket, local_addr).await {
                warn!("Failed to send SOCKS5 reply: {}", e);
                return;
            }

            if let Err(e) = tunnel_copy(socket, upstream, config.tunnel_buffer_size).await {
                debug!("Tunnel closed: {}", e);
            }
        }
        Ok(Err(e)) => {
            let rep = match e.kind() {
                std::io::ErrorKind::ConnectionRefused => SOCKS5_REP_CONN_REFUSED,
                std::io::ErrorKind::TimedOut => SOCKS5_REP_TTL_EXPIRED,
                std::io::ErrorKind::HostUnreachable => SOCKS5_REP_HOST_UNREACHABLE,
                std::io::ErrorKind::NetworkUnreachable => SOCKS5_REP_NET_UNREACHABLE,
                _ => SOCKS5_REP_GENERAL_FAILURE,
            };
            let _ = send_socks_reply(socket, rep, SOCKS5_ATYP_IPV4, "", 0).await;
        }
        Err(_) => {
            let _ = send_socks_reply(socket, SOCKS5_REP_TTL_EXPIRED, SOCKS5_ATYP_IPV4, "", 0).await;
        }
    }
}

async fn send_socks5_reply_connected(socket: &mut TcpStream, addr: SocketAddr) -> std::io::Result<()> {
    let reply = match addr.ip() {
        IpAddr::V4(ip) => {
            let mut r = vec![SOCKS5_VERSION, SOCKS5_REP_SUCCESS, 0x00, SOCKS5_ATYP_IPV4];
            r.extend_from_slice(&ip.octets());
            r.extend_from_slice(&addr.port().to_be_bytes());
            r
        }
        IpAddr::V6(ip) => {
            let mut r = vec![SOCKS5_VERSION, SOCKS5_REP_SUCCESS, 0x00, SOCKS5_ATYP_IPV6];
            r.extend_from_slice(ip.octets());
            r.extend_from_slice(&addr.port().to_be_bytes());
            r
        }
    };
    socket.write_all(&reply).await
}

async fn send_socks_reply(socket: &mut TcpStream, rep: u8, atyp: u8, host: &str, port: u16) -> std::io::Result<()> {
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
        SOCKS5_ATYP_IPV6 => {
            reply.extend_from_slice(&[0; 16]);
            reply.extend_from_slice(&port.to_be_bytes());
        }
        _ => {}
    }

    socket.write_all(&reply).await
}

async fn tunnel_copy(client: TcpStream, upstream: TcpStream, buffer_size: usize) -> std::io::Result<()> {
    let (cr, cw) = tokio::io::split(client);
    let (ur, uw) = tokio::io::split(upstream);

    let half_buffer = (buffer_size / 2).max(8192);

    let c2u = spawn(async move {
        let mut client_read = cr;
        let mut upstream_write = uw;
        let mut buffer = vec![0u8; half_buffer];

        loop {
            let n = client_read.read(&mut buffer).await?;
            if n == 0 {
                upstream_write.shutdown().await?;
                break;
            }
            upstream_write.write_all(&buffer[..n]).await?;
        }
        Ok::<_, std::io::Error>(())
    });

    let u2c = spawn(async move {
        let mut upstream_read = ur;
        let mut client_write = cw;
        let mut buffer = vec![0u8; half_buffer];

        loop {
            let n = upstream_read.read(&mut buffer).await?;
            if n == 0 {
                client_write.shutdown().await?;
                break;
            }
            client_write.write_all(&buffer[..n]).await?;
        }
        Ok::<_, std::io::Error>(())
    });

    let _ = tokio::join!(c2u, u2c);
    Ok(())
}

async fn write_response(socket: &mut TcpStream, status: u16, message: &str) -> std::io::Result<()> {
    let body = message.replace("\r\n", " ");
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.split_whitespace().collect::<Vec<_>>().join(""),
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await?;
    Ok(())
}