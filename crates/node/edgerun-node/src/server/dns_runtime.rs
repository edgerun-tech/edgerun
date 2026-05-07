use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;

use edgerun_protocols::dns::{
    dns_tcp_frame_len, encode_dns_tcp_frame, parse_dns_message_bounded, validate_name, DnsMessage,
    DnsRecord, DnsRecordType, DnsResponseCode, DnsZone,
};
use edgerun_rt::{
    AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, AsyncWriteExt,
    CancellationToken, RwLock,
};

#[cfg(not(target_os = "none"))]
use std::io;
#[cfg(not(target_os = "none"))]
use std::net::{IpAddr, SocketAddr};

#[cfg(target_os = "none")]
use edgerun_rt::io;
#[cfg(target_os = "none")]
type IpAddr = core::net::IpAddr;

const MAX_UDP_RESPONSE: usize = 512;

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
            (Some(Arc::new(bind_udp(addr)?)), Some(Arc::new(bind_tcp(addr)?)))
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

    pub async fn shutdown(&self) {}

    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        let tcp_shutdown = shutdown.clone();
        let tcp_listener = Arc::clone(&self.tcp_listener);
        let tcp_state = Arc::clone(&self.state);
        let tcp_limiter = self.rate_limiter.clone();
        let tcp_task = edgerun_rt::spawn(async move {
            tcp_accept_loop(tcp_listener, tcp_state, tcp_limiter, tcp_shutdown).await;
            Ok::<(), io::Error>(())
        });

        if let (Some(udp6), Some(tcp6)) = (&self.udp_socket_ipv6, &self.tcp_listener_ipv6) {
            let udp6 = Arc::clone(udp6);
            let tcp6 = Arc::clone(tcp6);
            let state = Arc::clone(&self.state);
            let limiter = self.rate_limiter.clone();
            let udp_shutdown = shutdown.clone();
            edgerun_rt::spawn(async move {
                udp_loop(udp6, state, limiter, udp_shutdown).await;
                Ok::<(), io::Error>(())
            });

            let state = Arc::clone(&self.state);
            let limiter = self.rate_limiter.clone();
            let tcp_shutdown = shutdown.clone();
            edgerun_rt::spawn(async move {
                tcp_accept_loop(tcp6, state, limiter, tcp_shutdown).await;
                Ok::<(), io::Error>(())
            });
        }

        udp_loop(
            Arc::clone(&self.udp_socket),
            Arc::clone(&self.state),
            self.rate_limiter.clone(),
            shutdown,
        )
        .await;
        let _ = tcp_task.await;
        Ok(())
    }
}

async fn udp_loop(
    socket: Arc<AsyncUdpSocket>,
    state: Arc<DnsState>,
    rate_limiter: RateLimiter,
    shutdown: CancellationToken,
) {
    while !shutdown.is_cancelled() {
        let mut buf = [0u8; 4096];
        let (n, src) = match socket.recv_from(&mut buf).await {
            Ok(received) => received,
            Err(_) => {
                edgerun_rt::sleep(Duration::from_millis(10)).await;
                continue;
            }
        };

        if !rate_limiter.allow(src.ip()) {
            let response = DnsMessage::response(0, DnsResponseCode::Refused, Vec::new());
            let _ = socket.send_to(&response.to_wire(), src).await;
            continue;
        }

        let query_wire = buf[..n].to_vec();
        let socket = Arc::clone(&socket);
        let state = Arc::clone(&state);
        edgerun_rt::spawn(async move {
            let response = match handle_query(&query_wire, &state).await {
                Ok((response_wire, needs_tcp)) => {
                    udp_response_wire(&query_wire, response_wire, needs_tcp)
                }
                Err(()) => DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            };
            let _ = socket.send_to(&response, src).await;
            Ok::<(), io::Error>(())
        });
    }
}

async fn tcp_accept_loop(
    listener: Arc<AsyncTcpListener>,
    state: Arc<DnsState>,
    rate_limiter: RateLimiter,
    shutdown: CancellationToken,
) {
    while !shutdown.is_cancelled() {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let state = Arc::clone(&state);
                let rate_limiter = rate_limiter.clone();
                edgerun_rt::spawn(async move {
                    let _ = handle_tcp_connection(stream, peer, state, rate_limiter).await;
                    Ok::<(), io::Error>(())
                });
            }
            Err(_) => edgerun_rt::sleep(Duration::from_millis(10)).await,
        }
    }
}

async fn handle_tcp_connection(
    mut stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    state: Arc<DnsState>,
    rate_limiter: RateLimiter,
) -> io::Result<()> {
    loop {
        let mut len_buf = [0u8; 2];
        if let Err(error) = stream.read_exact(&mut len_buf).await {
            return match error {
                edgerun_rt::IoError::UnexpectedEof => Ok(()),
                other => Err(rt_io_error(other)),
            };
        }
        let msg_len = dns_tcp_frame_len(len_buf).map_err(dns_frame_error)?;
        let mut query = alloc::vec![0u8; msg_len];
        stream.read_exact(&mut query).await.map_err(rt_io_error)?;

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
    let query = parse_dns_message_bounded(wire).map_err(|_| ())?;
    if query.header.is_response {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            false,
        ));
    }

    let question = match query.questions.first() {
        Some(question) => question,
        None => {
            return Ok((
                DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new())
                    .to_wire(),
                false,
            ))
        }
    };

    if question.qtype == DnsRecordType::AXFR {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::Refused, Vec::new()).to_wire(),
            false,
        ));
    }

    let qname = question.name.to_lowercase();
    if validate_name(&qname).is_err() {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            false,
        ));
    }

    let zones = state.zones.read();
    let answers = resolve(&qname, question.qtype, &zones);
    let mut response = if answers.is_empty() {
        DnsMessage::response(query.header.id, DnsResponseCode::NXDomain, Vec::new())
    } else {
        DnsMessage::response(query.header.id, DnsResponseCode::NoError, answers)
    };
    response.questions = query.questions.clone();
    response.header.question_count = response.questions.len() as u16;
    response.header.answer_count = response.answers.len() as u16;

    let wire = response.to_wire();
    let needs_tcp = wire.len() > MAX_UDP_RESPONSE;
    Ok((wire, needs_tcp))
}

fn resolve(
    qname: &str,
    qtype: DnsRecordType,
    zones: &BTreeMap<String, DnsZone>,
) -> Vec<DnsRecord> {
    best_matching_zone(qname, zones)
        .and_then(|zone| zone.resolve(qname, qtype))
        .unwrap_or_default()
}

fn best_matching_zone<'a>(
    qname: &str,
    zones: &'a BTreeMap<String, DnsZone>,
) -> Option<&'a DnsZone> {
    let qname = qname.trim_end_matches('.').to_ascii_lowercase();
    zones
        .iter()
        .filter(|(origin, _)| qname == **origin || qname.ends_with(&format!(".{origin}")))
        .max_by_key(|(origin, _)| origin.len())
        .map(|(_, zone)| zone)
}

fn udp_response_wire(query_wire: &[u8], response_wire: Vec<u8>, needs_tcp: bool) -> Vec<u8> {
    if !needs_tcp {
        return response_wire;
    }

    match parse_dns_message_bounded(query_wire) {
        Ok(query) => {
            let mut response =
                DnsMessage::response(query.header.id, DnsResponseCode::NoError, Vec::new());
            response.header.truncated = true;
            response.questions = query.questions;
            response.header.question_count = response.questions.len() as u16;
            response.to_wire()
        }
        Err(_) => response_wire,
    }
}

#[derive(Clone)]
struct RateLimiter {
    max_qps: u32,
    state: Arc<edgerun_rt::Mutex<BTreeMap<IpAddr, (u32, edgerun_rt::Instant)>>>,
}

impl RateLimiter {
    fn new(max_qps: u32) -> Self {
        Self {
            max_qps,
            state: Arc::new(edgerun_rt::Mutex::new(BTreeMap::new())),
        }
    }

    fn allow(&self, addr: IpAddr) -> bool {
        if self.max_qps == 0 {
            return true;
        }
        let mut state = self.state.lock();
        let now = edgerun_rt::Instant::now();
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
    AsyncUdpSocket::bind(addr).map_err(runtime_io_error)
}

fn bind_tcp(addr: &str) -> io::Result<AsyncTcpListener> {
    AsyncTcpListener::bind(addr).map_err(runtime_io_error)
}

fn runtime_io_error(error: edgerun_rt::IoError) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{error}"))
}

fn rt_io_error(error: edgerun_rt::IoError) -> io::Error {
    match error {
        edgerun_rt::IoError::UnexpectedEof => {
            io::Error::new(io::ErrorKind::UnexpectedEof, "unexpected end of file")
        }
        edgerun_rt::IoError::WriteZero => io::Error::new(io::ErrorKind::WriteZero, "write zero"),
        edgerun_rt::IoError::Other(message) => io::Error::new(io::ErrorKind::Other, message),
    }
}

fn dns_frame_error(error: edgerun_protocols::dns::DnsTcpFrameError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}"))
}
