use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;

use crate::network::{HostSocketTransport, TransportAddress, TransportError};
use crate::rt::{
    AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, AsyncWriteExt,
    CancellationToken, RwLock,
};
use edgerun_protocols::dns::{
    DnsMessage, DnsRecordData, DnsRecordType, DnsResponseCode, DnsZone, dns_tcp_frame_len,
    encode_dns_tcp_frame, handle_query_without_forwarding, resolve, udp_response_wire,
};

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const TCP_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

#[cfg(not(target_os = "none"))]
use std::io;
#[cfg(not(target_os = "none"))]
use std::net::{IpAddr, SocketAddr};

#[cfg(target_os = "none")]
use crate::rt::io;
#[cfg(target_os = "none")]
type IpAddr = core::net::IpAddr;

#[derive(Debug, Clone)]
pub struct DnsRuntimeConfig {
    pub bind_addr: String,
    pub bind_addr_ipv6: Option<String>,
    pub default_ttl: u32,
    pub rate_limit_qps: u32,
}

impl Default for DnsRuntimeConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:53".to_string(),
            bind_addr_ipv6: None,
            default_ttl: 3600,
            rate_limit_qps: 0,
        }
    }
}

#[derive(Clone)]
pub struct DnsRuntime {
    udp_socket: Arc<AsyncUdpSocket>,
    tcp_listener: Arc<AsyncTcpListener>,
    state: Arc<DnsState>,
    rate_limiter: RateLimiter,
    udp_socket_ipv6: Option<Arc<AsyncUdpSocket>>,
    tcp_listener_ipv6: Option<Arc<AsyncTcpListener>>,
}

struct DnsState {
    zones: RwLock<BTreeMap<String, DnsZone>>,
    default_ttl: u32,
}

impl DnsRuntime {
    pub fn new(config: DnsRuntimeConfig) -> io::Result<Self> {
        let udp_socket = Arc::new(bind_udp(&config.bind_addr)?);
        let tcp_listener = Arc::new(bind_tcp(&config.bind_addr)?);

        let (udp_socket_ipv6, tcp_listener_ipv6) = if let Some(ref addr) = config.bind_addr_ipv6 {
            (
                Some(Arc::new(bind_udp(addr)?)),
                Some(Arc::new(bind_tcp(addr)?)),
            )
        } else {
            (None, None)
        };

        Ok(Self {
            udp_socket,
            tcp_listener,
            state: Arc::new(DnsState {
                zones: RwLock::new(BTreeMap::new()),
                default_ttl: config.default_ttl,
            }),
            rate_limiter: RateLimiter::new(config.rate_limit_qps),
            udp_socket_ipv6,
            tcp_listener_ipv6,
        })
    }

    pub async fn add_zone(&self, mut zone: DnsZone) {
        zone.set_default_ttl(self.state.default_ttl);
        let origin = zone.origin.clone();
        self.state.zones.write().insert(origin, zone);
    }

    pub async fn remove_zone(&self, origin: &str) {
        self.state.zones.write().remove(origin);
    }

    pub async fn zone_count(&self) -> usize {
        self.state.zones.read().len()
    }

    pub async fn zone_names(&self) -> Vec<String> {
        self.state.zones.read().keys().cloned().collect()
    }

    pub async fn query_txt(&self, name: &str) -> Vec<String> {
        let zones = self.state.zones.read();
        resolve(name, DnsRecordType::TXT, &zones)
            .into_iter()
            .filter_map(|record| match record.data {
                DnsRecordData::TXT(value) => Some(value),
                _ => None,
            })
            .collect()
    }

    pub async fn query_mx(&self, name: &str) -> Vec<(u16, String)> {
        let zones = self.state.zones.read();
        let mut records: Vec<(u16, String)> = resolve(name, DnsRecordType::MX, &zones)
            .into_iter()
            .filter_map(|record| match record.data {
                DnsRecordData::MX { priority, exchange } => Some((priority, exchange)),
                _ => None,
            })
            .collect();
        records.sort_by_key(|(priority, _)| *priority);
        records
    }

    pub async fn shutdown(&self) {}

    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        let mut tasks: Vec<crate::rt::JoinHandle<io::Result<()>>> = Vec::new();

        let tcp_shutdown = shutdown.clone();
        let tcp_listener = Arc::clone(&self.tcp_listener);
        let tcp_state = Arc::clone(&self.state);
        let tcp_limiter = self.rate_limiter.clone();
        tasks.push(crate::rt::spawn(async move {
            tcp_accept_loop(tcp_listener, tcp_state, tcp_limiter, tcp_shutdown).await
        }));

        if let (Some(udp6), Some(tcp6)) = (&self.udp_socket_ipv6, &self.tcp_listener_ipv6) {
            let udp6 = Arc::clone(udp6);
            let tcp6 = Arc::clone(tcp6);
            let state = Arc::clone(&self.state);
            let limiter = self.rate_limiter.clone();
            let udp_shutdown = shutdown.clone();
            tasks.push(crate::rt::spawn(async move {
                udp_loop(udp6, state, limiter, udp_shutdown).await
            }));

            let state = Arc::clone(&self.state);
            let limiter = self.rate_limiter.clone();
            let tcp_shutdown = shutdown.clone();
            tasks.push(crate::rt::spawn(async move {
                tcp_accept_loop(tcp6, state, limiter, tcp_shutdown).await
            }));
        }

        let main_result = udp_loop(
            Arc::clone(&self.udp_socket),
            Arc::clone(&self.state),
            self.rate_limiter.clone(),
            shutdown.clone(),
        )
        .await;

        if main_result.is_err() {
            shutdown.cancel();
        }

        for task in tasks {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => return Err(error),
                Err(error) => return Err(join_error(error)),
            }
        }

        main_result
    }
}

async fn udp_loop(
    socket: Arc<AsyncUdpSocket>,
    state: Arc<DnsState>,
    rate_limiter: RateLimiter,
    shutdown: CancellationToken,
) -> io::Result<()> {
    while !shutdown.is_cancelled() {
        let mut buf = [0u8; 4096];
        let (n, src) = match socket.recv_from(&mut buf).await {
            Ok(received) => received,
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "DNS UDP receive failed",
                ));
            }
        };

        if !rate_limiter.allow(src.ip()) {
            let response = DnsMessage::response(0, DnsResponseCode::Refused, Vec::new());
            socket
                .send_to(&response.to_wire(), src)
                .await
                .map_err(rt_io_error)?;
            continue;
        }

        let query_wire = buf[..n].to_vec();
        let response = match handle_query(&query_wire, &state).await {
            Ok((response_wire, needs_tcp)) => {
                udp_response_wire(&query_wire, response_wire, needs_tcp)
            }
            Err(()) => DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new()).to_wire(),
        };
        socket.send_to(&response, src).await.map_err(rt_io_error)?;
    }
    Ok(())
}

async fn tcp_accept_loop(
    listener: Arc<AsyncTcpListener>,
    state: Arc<DnsState>,
    rate_limiter: RateLimiter,
    shutdown: CancellationToken,
) -> io::Result<()> {
    while !shutdown.is_cancelled() {
        match crate::rt::timeout(ACCEPT_POLL_INTERVAL, listener.accept()).await {
            Err(_) => continue,
            Ok(Ok((stream, peer))) => {
                let state = Arc::clone(&state);
                let rate_limiter = rate_limiter.clone();
                crate::rt::spawn(async move {
                    if let Err(error) =
                        handle_tcp_connection(stream, peer, state, rate_limiter).await
                    {
                        crate::node_warn!("dns tcp session failed: {}", error);
                    }
                });
            }
            Ok(Err(error)) => return Err(rt_io_error(error)),
        }
    }
    Ok(())
}

async fn handle_tcp_connection(
    mut stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    state: Arc<DnsState>,
    rate_limiter: RateLimiter,
) -> io::Result<()> {
    loop {
        let mut len_buf = [0u8; 2];
        let len_read = crate::rt::timeout(TCP_IDLE_TIMEOUT, stream.read_exact(&mut len_buf)).await;
        let len_read = match len_read {
            Ok(result) => result,
            Err(_) => return Ok(()),
        };
        if let Err(error) = len_read {
            return match error {
                crate::rt::IoError::UnexpectedEof => Ok(()),
                other => Err(rt_io_error(other)),
            };
        }
        let msg_len = dns_tcp_frame_len(len_buf).map_err(dns_frame_error)?;
        let mut query = alloc::vec![0u8; msg_len];
        crate::rt::timeout(TCP_IDLE_TIMEOUT, stream.read_exact(&mut query))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "dns tcp query timed out"))?
            .map_err(rt_io_error)?;

        let response = if rate_limiter.allow(peer.ip()) {
            match handle_query(&query, &state).await {
                Ok((wire, _)) => wire,
                Err(()) => DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            }
        } else {
            DnsMessage::response(0, DnsResponseCode::Refused, Vec::new()).to_wire()
        };

        let frame = encode_dns_tcp_frame(&response).map_err(dns_frame_error)?;
        stream.write_all(&frame).await.map_err(rt_io_error)?;
    }
}

async fn handle_query(wire: &[u8], state: &DnsState) -> Result<(Vec<u8>, bool), ()> {
    let zones = state.zones.read();
    handle_query_without_forwarding(wire, &zones).map_err(|_| ())
}

#[derive(Clone)]
struct RateLimiter {
    max_qps: u32,
    state: Arc<crate::rt::Mutex<BTreeMap<IpAddr, (u32, crate::rt::Instant)>>>,
}

impl RateLimiter {
    fn new(max_qps: u32) -> Self {
        Self {
            max_qps,
            state: Arc::new(crate::rt::Mutex::new(BTreeMap::new())),
        }
    }

    fn allow(&self, addr: IpAddr) -> bool {
        if self.max_qps == 0 {
            return true;
        }
        let mut state = self.state.lock();
        let now = crate::rt::Instant::now();
        let (tokens, last) = state.entry(addr).or_insert((self.max_qps, now));
        let elapsed = (now - *last).as_secs_f64();
        *tokens = (*tokens as f64 + elapsed * self.max_qps as f64).min(self.max_qps as f64) as u32;
        *last = now;
        if *tokens == 0 {
            return false;
        }
        *tokens -= 1;
        true
    }
}

fn bind_udp(addr: &str) -> io::Result<AsyncUdpSocket> {
    HostSocketTransport
        .bind_datagram_now(&TransportAddress::host_datagram(addr.as_bytes().to_vec()))
        .map_err(transport_io_error)
}

fn bind_tcp(addr: &str) -> io::Result<AsyncTcpListener> {
    HostSocketTransport
        .bind_stream_now(&TransportAddress::host_stream(addr.as_bytes().to_vec()))
        .map_err(transport_io_error)
}

fn transport_io_error(error: TransportError) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{error}"))
}

fn rt_io_error(error: crate::rt::IoError) -> io::Error {
    match error {
        crate::rt::IoError::UnexpectedEof => {
            io::Error::new(io::ErrorKind::UnexpectedEof, "unexpected end of file")
        }
        crate::rt::IoError::WriteZero => io::Error::new(io::ErrorKind::WriteZero, "write zero"),
        crate::rt::IoError::Other(message) => io::Error::new(io::ErrorKind::Other, message),
    }
}

fn dns_frame_error(error: edgerun_protocols::dns::DnsTcpFrameError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}"))
}

fn join_error(error: crate::rt::JoinError) -> io::Error {
    let _ = error;
    io::Error::new(io::ErrorKind::Other, "DNS runtime task failed")
}
