//! Bare-target async network compatibility for crates migrating off hosted runtimes.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::future::Future;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::pin::Pin;
use core::sync::atomic::{AtomicU16, Ordering};
use core::task::{Context, Poll};

use crate::io::{AsyncRead, AsyncWrite, IoError, Result as IoResult};
use crate::ip::{
    checksum, ip_checksum, EthHeader, IpHeader, TcpHeader, ETH_TYPE_ARP, ETH_TYPE_IPV4,
    IP_PROTO_TCP, IP_PROTO_UDP, TCP_FLAG_ACK, TCP_FLAG_FIN, TCP_FLAG_PSH, TCP_FLAG_RST,
    TCP_FLAG_SYN,
};
use crate::sync::Mutex;

const MAX_STREAMS: usize = 8;
const MTU: usize = 1514;
const TCP_HEADER_LEN: usize = 20;
const UDP_HEADER_LEN: usize = 8;
const IP_HEADER_LEN: usize = 20;
const ETH_HEADER_LEN: usize = 14;
const TCP_PACKET_HEADER_LEN: usize = ETH_HEADER_LEN + IP_HEADER_LEN + TCP_HEADER_LEN;
const UDP_PACKET_HEADER_LEN: usize = ETH_HEADER_LEN + IP_HEADER_LEN + UDP_HEADER_LEN;
const CONNECT_POLL_BUDGET: usize = 128;
const STREAM_RX_BUF_LEN: usize = 65536;
const MAX_UDP_PACKETS: usize = 8;
const UDP_PAYLOAD_LEN: usize = 1472;

static DRIVER: Mutex<Option<&'static dyn BareNetDriver>> = Mutex::new(None);
static STREAMS: Mutex<[StreamState; MAX_STREAMS]> = Mutex::new([StreamState::empty(); MAX_STREAMS]);
static UDP_PACKETS: Mutex<[UdpPacket; MAX_UDP_PACKETS]> =
    Mutex::new([UdpPacket::empty(); MAX_UDP_PACKETS]);
static NEXT_PORT: AtomicU16 = AtomicU16::new(49152);

pub trait BareNetDriver: Sync {
    fn local_ipv4(&self) -> [u8; 4];
    fn gateway_ipv4(&self) -> [u8; 4];
    fn local_mac(&self) -> [u8; 6];
    fn lookup_arp(&self, ipv4: [u8; 4]) -> Option<[u8; 6]>;
    fn send_frame(&self, frame: &[u8]) -> bool;
    fn recv_frame(&self, out: &mut [u8]) -> Option<usize>;
}

pub fn install_bare_net_driver(driver: &'static dyn BareNetDriver) {
    *DRIVER.lock() = Some(driver);
}

fn unavailable() -> IoError {
    IoError::Other("bare async network operation is unavailable")
}

fn unspecified_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
}

#[derive(Clone, Copy)]
struct StreamState {
    active: bool,
    established: bool,
    closed: bool,
    local_ip: [u8; 4],
    remote_ip: [u8; 4],
    remote_mac: [u8; 6],
    local_port: u16,
    remote_port: u16,
    seq: u32,
    ack: u32,
    rx: [u8; STREAM_RX_BUF_LEN],
    rx_start: usize,
    rx_len: usize,
}

#[derive(Clone, Copy)]
struct UdpPacket {
    used: bool,
    src_ip: [u8; 4],
    src_port: u16,
    dst_ip: [u8; 4],
    dst_port: u16,
    len: usize,
    payload: [u8; UDP_PAYLOAD_LEN],
}

impl UdpPacket {
    const fn empty() -> Self {
        Self {
            used: false,
            src_ip: [0; 4],
            src_port: 0,
            dst_ip: [0; 4],
            dst_port: 0,
            len: 0,
            payload: [0; UDP_PAYLOAD_LEN],
        }
    }
}

impl StreamState {
    const fn empty() -> Self {
        Self {
            active: false,
            established: false,
            closed: false,
            local_ip: [0; 4],
            remote_ip: [0; 4],
            remote_mac: [0; 6],
            local_port: 0,
            remote_port: 0,
            seq: 0,
            ack: 0,
            rx: [0; STREAM_RX_BUF_LEN],
            rx_start: 0,
            rx_len: 0,
        }
    }

    fn enqueue_rx(&mut self, bytes: &[u8]) -> usize {
        let free = self.rx.len().saturating_sub(self.rx_len);
        let copy_len = core::cmp::min(free, bytes.len());
        if copy_len == 0 {
            return 0;
        }
        let write_start = (self.rx_start + self.rx_len) % self.rx.len();
        let first = core::cmp::min(copy_len, self.rx.len() - write_start);
        let second = copy_len - first;
        self.rx[write_start..write_start + first].copy_from_slice(&bytes[..first]);
        if second > 0 {
            self.rx[..second].copy_from_slice(&bytes[first..copy_len]);
        }
        self.rx_len += copy_len;
        copy_len
    }

    fn read_rx(&mut self, out: &mut [u8]) -> usize {
        let copy_len = core::cmp::min(out.len(), self.rx_len);
        if copy_len == 0 {
            return 0;
        }
        let first = core::cmp::min(copy_len, self.rx.len() - self.rx_start);
        let second = copy_len - first;
        out[..first].copy_from_slice(&self.rx[self.rx_start..self.rx_start + first]);
        if second > 0 {
            out[first..copy_len].copy_from_slice(&self.rx[..second]);
        }
        self.rx_start = (self.rx_start + copy_len) % self.rx.len();
        self.rx_len -= copy_len;
        copy_len
    }

    fn advertised_window(&self) -> u16 {
        let free = self.rx.len().saturating_sub(self.rx_len);
        core::cmp::min(free, u16::MAX as usize) as u16
    }
}

#[derive(Clone)]
pub struct AsyncTcpStream {
    id: usize,
}

impl AsyncTcpStream {
    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        let streams = STREAMS.lock();
        let stream = streams.get(self.id).ok_or(unavailable())?;
        Ok(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from(stream.local_ip)),
            stream.local_port,
        ))
    }

    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        let streams = STREAMS.lock();
        let stream = streams.get(self.id).ok_or(unavailable())?;
        Ok(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from(stream.remote_ip)),
            stream.remote_port,
        ))
    }

    pub fn split(self: Arc<Self>) -> (Arc<Self>, Arc<Self>) {
        (self.clone(), self)
    }
}

impl Drop for AsyncTcpStream {
    fn drop(&mut self) {
        let mut streams = STREAMS.lock();
        let Some(stream) = streams.get_mut(self.id) else {
            return;
        };
        *stream = StreamState::empty();
    }
}

impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        poll_read_stream(self.id, buf)
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        let Some(driver) = *DRIVER.lock() else {
            return Poll::Ready(Err(unavailable()));
        };
        poll_driver();
        let mut streams = STREAMS.lock();
        let Some(stream) = streams.get_mut(self.id) else {
            return Poll::Ready(Err(unavailable()));
        };
        if !stream.established || stream.closed {
            return Poll::Ready(Err(IoError::WriteZero));
        }
        let copy_len = core::cmp::min(buf.len(), MTU - TCP_PACKET_HEADER_LEN);
        if copy_len == 0 {
            return Poll::Ready(Ok(0));
        }
        if send_tcp_segment(
            driver,
            stream,
            TCP_FLAG_PSH | TCP_FLAG_ACK,
            &buf[..copy_len],
        ) {
            stream.seq = stream.seq.wrapping_add(copy_len as u32);
            Poll::Ready(Ok(copy_len))
        } else {
            Poll::Ready(Err(IoError::WriteZero))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let Some(driver) = *DRIVER.lock() else {
            let mut streams = STREAMS.lock();
            if let Some(stream) = streams.get_mut(self.id) {
                stream.closed = true;
                stream.active = false;
            }
            return Poll::Ready(Ok(()));
        };
        let mut streams = STREAMS.lock();
        if let Some(stream) = streams.get_mut(self.id) {
            let _ = send_tcp_segment(driver, stream, TCP_FLAG_FIN | TCP_FLAG_ACK, &[]);
            stream.closed = true;
            stream.active = false;
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for Arc<AsyncTcpStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        poll_read_stream(self.id, buf)
    }
}

impl AsyncWrite for Arc<AsyncTcpStream> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        let Some(driver) = *DRIVER.lock() else {
            return Poll::Ready(Err(unavailable()));
        };
        poll_driver();
        let mut streams = STREAMS.lock();
        let Some(stream) = streams.get_mut(self.id) else {
            return Poll::Ready(Err(unavailable()));
        };
        if !stream.established || stream.closed {
            return Poll::Ready(Err(IoError::WriteZero));
        }
        let copy_len = core::cmp::min(buf.len(), MTU - TCP_PACKET_HEADER_LEN);
        if copy_len == 0 {
            return Poll::Ready(Ok(0));
        }
        if send_tcp_segment(
            driver,
            stream,
            TCP_FLAG_PSH | TCP_FLAG_ACK,
            &buf[..copy_len],
        ) {
            stream.seq = stream.seq.wrapping_add(copy_len as u32);
            Poll::Ready(Ok(copy_len))
        } else {
            Poll::Ready(Err(IoError::WriteZero))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        let Some(driver) = *DRIVER.lock() else {
            let mut streams = STREAMS.lock();
            if let Some(stream) = streams.get_mut(self.id) {
                stream.closed = true;
                stream.active = false;
            }
            return Poll::Ready(Ok(()));
        };
        let mut streams = STREAMS.lock();
        if let Some(stream) = streams.get_mut(self.id) {
            let _ = send_tcp_segment(driver, stream, TCP_FLAG_FIN | TCP_FLAG_ACK, &[]);
            stream.closed = true;
            stream.active = false;
        }
        Poll::Ready(Ok(()))
    }
}

fn poll_read_stream(id: usize, buf: &mut [u8]) -> Poll<IoResult<usize>> {
    poll_driver();
    let driver = *DRIVER.lock();
    let mut streams = STREAMS.lock();
    let Some(stream) = streams.get_mut(id) else {
        return Poll::Ready(Err(unavailable()));
    };
    if stream.rx_len != 0 {
        let read = stream.read_rx(buf);
        if read != 0 {
            if let Some(driver) = driver {
                let _ = send_tcp_segment(driver, stream, TCP_FLAG_ACK, &[]);
            }
        }
        return Poll::Ready(Ok(read));
    }
    if stream.closed {
        *stream = StreamState::empty();
        return Poll::Ready(Ok(0));
    }
    Poll::Pending
}

pub struct ConnectFuture {
    target: String,
    stream_id: Option<usize>,
    sent_syn: bool,
}

impl ConnectFuture {
    pub fn new<A: ToString>(addrs: A) -> Self {
        Self {
            target: addrs.to_string(),
            stream_id: None,
            sent_syn: false,
        }
    }
}

impl Future for ConnectFuture {
    type Output = IoResult<Arc<AsyncTcpStream>>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(driver) = *DRIVER.lock() else {
            return Poll::Ready(Err(unavailable()));
        };
        let (remote_ip, remote_port) = match parse_ipv4_socket_addr(&self.target) {
            Some(value) => value,
            None => return Poll::Ready(Err(IoError::Other("bare TCP requires IPv4 socket addr"))),
        };
        if self.stream_id.is_none() {
            let routed_ip = route_ipv4(driver.local_ipv4(), driver.gateway_ipv4(), remote_ip);
            let remote_mac = driver.lookup_arp(routed_ip).unwrap_or([0xff; 6]);
            let mut streams = STREAMS.lock();
            let Some((id, stream)) = streams
                .iter_mut()
                .enumerate()
                .find(|(_, stream)| !stream.active)
            else {
                return Poll::Ready(Err(IoError::Other("bare TCP stream table full")));
            };
            *stream = StreamState {
                active: true,
                local_ip: driver.local_ipv4(),
                remote_ip,
                remote_mac,
                local_port: allocate_ephemeral_port(),
                remote_port,
                seq: initial_seq(remote_ip, remote_port),
                ..StreamState::empty()
            };
            self.stream_id = Some(id);
        }

        let Some(id) = self.stream_id else {
            return Poll::Ready(Err(unavailable()));
        };
        if !self.sent_syn {
            let mut streams = STREAMS.lock();
            let Some(stream) = streams.get_mut(id) else {
                return Poll::Ready(Err(unavailable()));
            };
            if !send_tcp_segment(driver, stream, TCP_FLAG_SYN, &[]) {
                stream.active = false;
                self.stream_id = None;
                return Poll::Ready(Err(IoError::WriteZero));
            }
            stream.seq = stream.seq.wrapping_add(1);
            self.sent_syn = true;
        }

        for _ in 0..CONNECT_POLL_BUDGET {
            poll_driver();
            let streams = STREAMS.lock();
            if streams
                .get(id)
                .map(|stream| stream.established)
                .unwrap_or(false)
            {
                return Poll::Ready(Ok(Arc::new(AsyncTcpStream { id })));
            }
        }
        Poll::Pending
    }
}

impl Drop for ConnectFuture {
    fn drop(&mut self) {
        let Some(id) = self.stream_id.take() else {
            return;
        };
        let mut streams = STREAMS.lock();
        let Some(stream) = streams.get_mut(id) else {
            return;
        };
        if !stream.established {
            *stream = StreamState::empty();
        }
    }
}

pub struct AsyncTcpListener;

impl AsyncTcpListener {
    pub fn bind<A>(_addr: A) -> IoResult<Self> {
        Ok(Self)
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub async fn accept(&self) -> IoResult<(Arc<AsyncTcpStream>, SocketAddr)> {
        Err(unavailable())
    }
}

pub struct AsyncUdpSocket;

impl AsyncUdpSocket {
    pub fn bind<A>(_addr: A) -> IoResult<Self> {
        Ok(Self)
    }

    pub fn from_std<T>(_socket: T) -> IoResult<Self> {
        Ok(Self)
    }

    pub fn connect<A>(&self, _addr: A) -> IoResult<()> {
        Ok(())
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        Ok(unspecified_addr())
    }

    pub fn set_broadcast(&self, _broadcast: bool) -> IoResult<()> {
        Ok(())
    }

    pub async fn send_to(&self, _buf: &[u8], _target: SocketAddr) -> IoResult<usize> {
        Err(unavailable())
    }

    pub async fn recv_from(&self, _buf: &mut [u8]) -> IoResult<(usize, SocketAddr)> {
        Err(unavailable())
    }

    pub fn send(&self, _buf: &[u8]) -> IoResult<usize> {
        Err(unavailable())
    }

    pub fn recv(&self, _buf: &mut [u8]) -> IoResult<usize> {
        Err(unavailable())
    }
}

pub fn bare_udp_send_to(
    local_ip: [u8; 4],
    local_port: u16,
    remote_ip: [u8; 4],
    remote_port: u16,
    payload: &[u8],
) -> IoResult<usize> {
    let Some(driver) = *DRIVER.lock() else {
        return Err(unavailable());
    };
    let source_ip = if local_ip == [0; 4] {
        driver.local_ipv4()
    } else {
        local_ip
    };
    if send_udp_datagram(
        driver,
        source_ip,
        local_port,
        remote_ip,
        remote_port,
        payload,
    ) {
        Ok(payload.len())
    } else {
        Err(IoError::WriteZero)
    }
}

pub fn bare_udp_recv_from(
    local_ip: [u8; 4],
    local_port: u16,
    out: &mut [u8],
) -> IoResult<(usize, [u8; 4], u16)> {
    poll_driver();
    let mut packets = UDP_PACKETS.lock();
    let Some((index, packet)) = packets.iter_mut().enumerate().find(|(_, packet)| {
        packet.used
            && packet.dst_port == local_port
            && (local_ip == [0; 4] || packet.dst_ip == local_ip || packet.dst_ip == [255; 4])
    }) else {
        return Err(IoError::Other("would block"));
    };
    let len = core::cmp::min(out.len(), packet.len);
    out[..len].copy_from_slice(&packet.payload[..len]);
    let src_ip = packet.src_ip;
    let src_port = packet.src_port;
    packets[index] = UdpPacket::empty();
    Ok((len, src_ip, src_port))
}

fn poll_driver() {
    let Some(driver) = *DRIVER.lock() else {
        return;
    };
    let mut frame = [0u8; MTU];
    while let Some(len) = driver.recv_frame(&mut frame) {
        handle_frame(driver, &frame[..len]);
    }
}

fn handle_frame(driver: &dyn BareNetDriver, frame: &[u8]) {
    if frame.len() < UDP_PACKET_HEADER_LEN {
        return;
    }
    let eth = EthHeader::from_slice(frame);
    if eth.ethertype == ETH_TYPE_ARP {
        return;
    }
    if eth.ethertype != ETH_TYPE_IPV4 {
        return;
    }
    let ip = IpHeader::from_slice(&frame[ETH_HEADER_LEN..]);
    if ip.dst != driver.local_ipv4() && ip.dst != [255; 4] {
        return;
    }
    let ip_header_len = usize::from(ip.ver_ihl & 0x0f) * 4;
    let ip_total_len = usize::from(ip.len);
    if ip.proto == IP_PROTO_UDP {
        handle_udp_frame(&ip, frame, ip_header_len, ip_total_len);
        return;
    }
    if ip.proto != IP_PROTO_TCP {
        return;
    }
    if ip_header_len < IP_HEADER_LEN
        || frame.len() < ETH_HEADER_LEN + ip_header_len + TCP_HEADER_LEN
        || ip_total_len < ip_header_len + TCP_HEADER_LEN
        || frame.len() < ETH_HEADER_LEN + ip_total_len
    {
        return;
    }
    let tcp_start = ETH_HEADER_LEN + ip_header_len;
    let tcp = TcpHeader::from_slice(&frame[tcp_start..]);
    let data_offset = usize::from(frame[tcp_start + 12] >> 4) * 4;
    if data_offset < TCP_HEADER_LEN || ip_total_len < ip_header_len + data_offset {
        return;
    }
    let payload_start = tcp_start + data_offset;
    let payload_end = ETH_HEADER_LEN + ip_total_len;
    let payload = &frame[payload_start..payload_end];
    let mut streams = STREAMS.lock();
    let Some(stream) = streams.iter_mut().find(|stream| {
        stream.active
            && stream.local_port == tcp.dst_port
            && stream.remote_port == tcp.src_port
            && stream.remote_ip == ip.src
    }) else {
        return;
    };

    if tcp.flags & TCP_FLAG_RST != 0 {
        stream.closed = true;
        stream.active = false;
        return;
    }
    if tcp.flags & TCP_FLAG_SYN != 0 && tcp.flags & TCP_FLAG_ACK != 0 && !stream.established {
        stream.ack = tcp.seq.wrapping_add(1);
        stream.established = true;
        let _ = send_tcp_segment(driver, stream, TCP_FLAG_ACK, &[]);
        return;
    }
    if !payload.is_empty() {
        if tcp.seq != stream.ack {
            let _ = send_tcp_segment(driver, stream, TCP_FLAG_ACK, &[]);
            return;
        }
        let queued = stream.enqueue_rx(payload);
        stream.ack = stream.ack.wrapping_add(queued as u32);
        let _ = send_tcp_segment(driver, stream, TCP_FLAG_ACK, &[]);
    }
    if tcp.flags & TCP_FLAG_FIN != 0
        && (payload.is_empty() || stream.ack == tcp.seq.wrapping_add(payload.len() as u32))
    {
        stream.ack = stream.ack.wrapping_add(1);
        stream.closed = true;
        stream.active = false;
        let _ = send_tcp_segment(driver, stream, TCP_FLAG_ACK, &[]);
    }
}

fn handle_udp_frame(ip: &IpHeader, frame: &[u8], ip_header_len: usize, ip_total_len: usize) {
    if ip_header_len < IP_HEADER_LEN
        || frame.len() < ETH_HEADER_LEN + ip_header_len + UDP_HEADER_LEN
        || ip_total_len < ip_header_len + UDP_HEADER_LEN
        || frame.len() < ETH_HEADER_LEN + ip_total_len
    {
        return;
    }
    let udp_start = ETH_HEADER_LEN + ip_header_len;
    let udp = crate::ip::UdpHeader::from_slice(&frame[udp_start..]);
    let udp_len = usize::from(udp.len);
    if udp_len < UDP_HEADER_LEN || ip_total_len < ip_header_len + udp_len {
        return;
    }
    let payload_start = udp_start + UDP_HEADER_LEN;
    let payload_len = core::cmp::min(udp_len - UDP_HEADER_LEN, UDP_PAYLOAD_LEN);
    if frame.len() < payload_start + payload_len {
        return;
    }

    let mut packets = UDP_PACKETS.lock();
    let Some(packet) = packets.iter_mut().find(|packet| !packet.used) else {
        return;
    };
    packet.used = true;
    packet.src_ip = ip.src;
    packet.src_port = udp.src_port;
    packet.dst_ip = ip.dst;
    packet.dst_port = udp.dst_port;
    packet.len = payload_len;
    packet.payload[..payload_len]
        .copy_from_slice(&frame[payload_start..payload_start + payload_len]);
}

fn send_tcp_segment(
    driver: &dyn BareNetDriver,
    stream: &StreamState,
    flags: u8,
    payload: &[u8],
) -> bool {
    let ip_len = IP_HEADER_LEN + TCP_HEADER_LEN + payload.len();
    let packet_len = ETH_HEADER_LEN + ip_len;
    if packet_len > MTU || ip_len > u16::MAX as usize {
        return false;
    }
    let mut packet = [0u8; MTU];
    EthHeader {
        dst: stream.remote_mac,
        src: driver.local_mac(),
        ethertype: ETH_TYPE_IPV4,
    }
    .to_slice(&mut packet[..ETH_HEADER_LEN]);
    let mut ip = IpHeader {
        ver_ihl: 0x45,
        tos: 0,
        len: ip_len as u16,
        ttl: 64,
        proto: IP_PROTO_TCP,
        checksum: 0,
        src: stream.local_ip,
        dst: stream.remote_ip,
    };
    ip.to_slice(&mut packet[ETH_HEADER_LEN..ETH_HEADER_LEN + IP_HEADER_LEN]);
    ip.checksum = ip_checksum(&packet[ETH_HEADER_LEN..ETH_HEADER_LEN + IP_HEADER_LEN]);
    packet[ETH_HEADER_LEN + 10..ETH_HEADER_LEN + 12].copy_from_slice(&ip.checksum.to_be_bytes());

    let tcp_start = ETH_HEADER_LEN + IP_HEADER_LEN;
    TcpHeader {
        src_port: stream.local_port,
        dst_port: stream.remote_port,
        seq: stream.seq,
        ack: stream.ack,
        flags,
        window: stream.advertised_window(),
        checksum: 0,
        urgent: 0,
    }
    .to_slice(&mut packet[tcp_start..tcp_start + TCP_HEADER_LEN]);
    packet[tcp_start + 12] = 5 << 4;
    packet[tcp_start + 13] = flags;
    packet[tcp_start + TCP_HEADER_LEN..packet_len].copy_from_slice(payload);
    let checksum = tcp_checksum(
        stream.local_ip,
        stream.remote_ip,
        &packet[tcp_start..tcp_start + TCP_HEADER_LEN + payload.len()],
    );
    packet[tcp_start + 16..tcp_start + 18].copy_from_slice(&checksum.to_be_bytes());
    driver.send_frame(&packet[..packet_len])
}

fn send_udp_datagram(
    driver: &dyn BareNetDriver,
    local_ip: [u8; 4],
    local_port: u16,
    remote_ip: [u8; 4],
    remote_port: u16,
    payload: &[u8],
) -> bool {
    let udp_len = UDP_HEADER_LEN + payload.len();
    let ip_len = IP_HEADER_LEN + udp_len;
    let packet_len = ETH_HEADER_LEN + ip_len;
    if packet_len > MTU || udp_len > u16::MAX as usize || ip_len > u16::MAX as usize {
        return false;
    }

    let routed_ip = route_ipv4(local_ip, driver.gateway_ipv4(), remote_ip);
    let dst_mac = if remote_ip == [255; 4] || routed_ip == [255; 4] {
        [0xff; 6]
    } else {
        driver.lookup_arp(routed_ip).unwrap_or([0xff; 6])
    };

    let mut packet = [0u8; MTU];
    EthHeader {
        dst: dst_mac,
        src: driver.local_mac(),
        ethertype: ETH_TYPE_IPV4,
    }
    .to_slice(&mut packet[..ETH_HEADER_LEN]);

    let mut ip = IpHeader {
        ver_ihl: 0x45,
        tos: 0,
        len: ip_len as u16,
        ttl: 64,
        proto: IP_PROTO_UDP,
        checksum: 0,
        src: local_ip,
        dst: remote_ip,
    };
    ip.to_slice(&mut packet[ETH_HEADER_LEN..ETH_HEADER_LEN + IP_HEADER_LEN]);
    ip.checksum = ip_checksum(&packet[ETH_HEADER_LEN..ETH_HEADER_LEN + IP_HEADER_LEN]);
    packet[ETH_HEADER_LEN + 10..ETH_HEADER_LEN + 12].copy_from_slice(&ip.checksum.to_be_bytes());

    let udp_start = ETH_HEADER_LEN + IP_HEADER_LEN;
    crate::ip::UdpHeader {
        src_port: local_port,
        dst_port: remote_port,
        len: udp_len as u16,
        checksum: 0,
    }
    .to_slice(&mut packet[udp_start..udp_start + UDP_HEADER_LEN]);
    packet[udp_start + UDP_HEADER_LEN..packet_len].copy_from_slice(payload);
    driver.send_frame(&packet[..packet_len])
}

fn tcp_checksum(src: [u8; 4], dst: [u8; 4], tcp: &[u8]) -> u16 {
    let mut pseudo = [0u8; 12 + MTU];
    pseudo[0..4].copy_from_slice(&src);
    pseudo[4..8].copy_from_slice(&dst);
    pseudo[8] = 0;
    pseudo[9] = IP_PROTO_TCP;
    pseudo[10..12].copy_from_slice(&(tcp.len() as u16).to_be_bytes());
    pseudo[12..12 + tcp.len()].copy_from_slice(tcp);
    checksum(&pseudo[..12 + tcp.len()])
}

fn parse_ipv4_socket_addr(input: &str) -> Option<([u8; 4], u16)> {
    let (host, port) = input.rsplit_once(':')?;
    let port = port.parse::<u16>().ok()?;
    let mut octets = [0u8; 4];
    let mut count = 0;
    for (index, part) in host.split('.').enumerate() {
        if index >= 4 {
            return None;
        }
        octets[index] = part.parse::<u8>().ok()?;
        count += 1;
    }
    (count == 4).then_some((octets, port))
}

fn route_ipv4(local: [u8; 4], gateway: [u8; 4], remote: [u8; 4]) -> [u8; 4] {
    if local[0] == remote[0] || gateway == [0; 4] {
        remote
    } else {
        gateway
    }
}

fn allocate_ephemeral_port() -> u16 {
    let port = NEXT_PORT.fetch_add(1, Ordering::AcqRel);
    if port < 49152 {
        NEXT_PORT.store(49153, Ordering::Release);
        49152
    } else {
        port
    }
}

fn initial_seq(remote_ip: [u8; 4], remote_port: u16) -> u32 {
    let mut out = 0x4552_0000u32;
    for byte in remote_ip {
        out = out.rotate_left(5) ^ u32::from(byte);
    }
    out ^ u32::from(remote_port)
}
