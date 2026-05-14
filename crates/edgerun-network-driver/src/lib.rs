#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "rtl8125")]
pub mod rtl8125;

pub const ETHERNET_HEADER_LEN: usize = 14;
pub const ETHERNET_MTU: u16 = 1500;
pub const ETHERNET_FRAME_LEN: usize = ETHERNET_HEADER_LEN + ETHERNET_MTU as usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameDriverKind {
    Unknown,
    InMemory,
    Rtl8125,
    StdUdp,
    Tap,
    VirtioNet,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameDriverStats {
    pub tx_frames: u64,
    pub tx_dropped: u64,
    pub rx_frames: u64,
    pub rx_dropped: u64,
    pub rx_errors: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameDriverInfo {
    pub name: &'static str,
    pub kind: FrameDriverKind,
    pub mac: [u8; 6],
    pub mtu: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameDriverError {
    InvalidBufferLength,
    InvalidFrameLength,
    NoTxDescriptor,
    NotInitialized,
    WouldBlock,
    Unsupported,
    Device,
}

pub type FrameDriverResult<T> = core::result::Result<T, FrameDriverError>;

pub trait FrameDevice {
    fn mac(&self) -> [u8; 6];
    fn mtu(&self) -> u16;

    fn name(&self) -> &'static str {
        "frame-device"
    }

    fn kind(&self) -> FrameDriverKind {
        FrameDriverKind::Unknown
    }

    fn info(&self) -> FrameDriverInfo {
        FrameDriverInfo {
            name: self.name(),
            kind: self.kind(),
            mac: self.mac(),
            mtu: self.mtu(),
        }
    }

    fn stats(&mut self) -> FrameDriverStats {
        FrameDriverStats::default()
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()>;
    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>>;

    fn send_frame(&mut self, frame: &[u8]) -> bool {
        self.try_send_frame(frame).is_ok()
    }

    fn recv_frame(&mut self, out: &mut [u8]) -> Option<usize> {
        self.try_recv_frame(out).ok().flatten()
    }
}

impl<T: FrameDevice + ?Sized> FrameDevice for &mut T {
    fn mac(&self) -> [u8; 6] {
        (**self).mac()
    }

    fn mtu(&self) -> u16 {
        (**self).mtu()
    }

    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn kind(&self) -> FrameDriverKind {
        (**self).kind()
    }

    fn stats(&mut self) -> FrameDriverStats {
        (**self).stats()
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        (**self).try_send_frame(frame)
    }

    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        (**self).try_recv_frame(out)
    }
}

#[cfg(feature = "alloc")]
impl<T: FrameDevice + ?Sized> FrameDevice for alloc::boxed::Box<T> {
    fn mac(&self) -> [u8; 6] {
        (**self).mac()
    }

    fn mtu(&self) -> u16 {
        (**self).mtu()
    }

    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn kind(&self) -> FrameDriverKind {
        (**self).kind()
    }

    fn stats(&mut self) -> FrameDriverStats {
        (**self).stats()
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        (**self).try_send_frame(frame)
    }

    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        (**self).try_recv_frame(out)
    }
}

pub struct InMemoryFrameDevice<const RX: usize, const TX: usize, const MAX_FRAME: usize> {
    mac: [u8; 6],
    mtu: u16,
    rx_frames: [[u8; MAX_FRAME]; RX],
    rx_lens: [usize; RX],
    rx_read: usize,
    rx_len: usize,
    tx_frames: [[u8; MAX_FRAME]; TX],
    tx_lens: [usize; TX],
    tx_read: usize,
    tx_len: usize,
    stats: FrameDriverStats,
}

impl<const RX: usize, const TX: usize, const MAX_FRAME: usize>
    InMemoryFrameDevice<RX, TX, MAX_FRAME>
{
    pub const fn new(mac: [u8; 6], mtu: u16) -> Self {
        Self {
            mac,
            mtu,
            rx_frames: [[0; MAX_FRAME]; RX],
            rx_lens: [0; RX],
            rx_read: 0,
            rx_len: 0,
            tx_frames: [[0; MAX_FRAME]; TX],
            tx_lens: [0; TX],
            tx_read: 0,
            tx_len: 0,
            stats: FrameDriverStats {
                tx_frames: 0,
                tx_dropped: 0,
                rx_frames: 0,
                rx_dropped: 0,
                rx_errors: 0,
            },
        }
    }

    pub fn push_rx_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        if frame.len() > MAX_FRAME {
            self.stats.rx_dropped = self.stats.rx_dropped.wrapping_add(1);
            return Err(FrameDriverError::InvalidFrameLength);
        }
        if self.rx_len >= RX {
            self.stats.rx_dropped = self.stats.rx_dropped.wrapping_add(1);
            return Err(FrameDriverError::NoTxDescriptor);
        }
        let idx = (self.rx_read + self.rx_len) % RX;
        self.rx_frames[idx][..frame.len()].copy_from_slice(frame);
        self.rx_lens[idx] = frame.len();
        self.rx_len += 1;
        Ok(())
    }

    pub fn pop_tx_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        if self.tx_len == 0 {
            return Ok(None);
        }
        let len = self.tx_lens[self.tx_read];
        if out.len() < len {
            return Err(FrameDriverError::InvalidBufferLength);
        }
        out[..len].copy_from_slice(&self.tx_frames[self.tx_read][..len]);
        self.tx_read = (self.tx_read + 1) % TX;
        self.tx_len -= 1;
        Ok(Some(len))
    }

    pub fn queued_rx_len(&self) -> usize {
        self.rx_len
    }

    pub fn queued_tx_len(&self) -> usize {
        self.tx_len
    }
}

impl<const RX: usize, const TX: usize, const MAX_FRAME: usize> FrameDevice
    for InMemoryFrameDevice<RX, TX, MAX_FRAME>
{
    fn mac(&self) -> [u8; 6] {
        self.mac
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn name(&self) -> &'static str {
        "in-memory-frame-device"
    }

    fn kind(&self) -> FrameDriverKind {
        FrameDriverKind::InMemory
    }

    fn stats(&mut self) -> FrameDriverStats {
        self.stats
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        if frame.len() > MAX_FRAME {
            self.stats.tx_dropped = self.stats.tx_dropped.wrapping_add(1);
            return Err(FrameDriverError::InvalidFrameLength);
        }
        if self.tx_len >= TX {
            self.stats.tx_dropped = self.stats.tx_dropped.wrapping_add(1);
            return Err(FrameDriverError::NoTxDescriptor);
        }
        let idx = (self.tx_read + self.tx_len) % TX;
        self.tx_frames[idx][..frame.len()].copy_from_slice(frame);
        self.tx_lens[idx] = frame.len();
        self.tx_len += 1;
        self.stats.tx_frames = self.stats.tx_frames.wrapping_add(1);
        Ok(())
    }

    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        if self.rx_len == 0 {
            return Ok(None);
        }
        let len = self.rx_lens[self.rx_read];
        if out.len() < len {
            return Err(FrameDriverError::InvalidBufferLength);
        }
        out[..len].copy_from_slice(&self.rx_frames[self.rx_read][..len]);
        self.rx_read = (self.rx_read + 1) % RX;
        self.rx_len -= 1;
        self.stats.rx_frames = self.stats.rx_frames.wrapping_add(1);
        Ok(Some(len))
    }
}

#[cfg(feature = "virtio")]
impl FrameDevice for edgerun_virtio::VirtNet {
    fn mac(&self) -> [u8; 6] {
        self.get_mac()
    }

    fn mtu(&self) -> u16 {
        self.mtu()
    }

    fn name(&self) -> &'static str {
        "virtio-net"
    }

    fn kind(&self) -> FrameDriverKind {
        FrameDriverKind::VirtioNet
    }

    fn stats(&mut self) -> FrameDriverStats {
        let stats = self.stats();
        FrameDriverStats {
            tx_frames: stats.tx_submitted as u64,
            tx_dropped: stats.tx_submitted.saturating_sub(stats.tx_completed) as u64,
            rx_frames: stats.rx_received as u64,
            rx_dropped: stats.rx_invalid.wrapping_add(stats.rx_empty) as u64,
            rx_errors: stats.rx_invalid as u64,
        }
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        self.try_send(frame).map_err(virtio_error)
    }

    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        self.try_recv(out).map_err(virtio_error)
    }
}

#[cfg(feature = "virtio")]
fn virtio_error(error: edgerun_virtio::VirtioError) -> FrameDriverError {
    match error {
        edgerun_virtio::VirtioError::InvalidBufferLength => FrameDriverError::InvalidBufferLength,
        edgerun_virtio::VirtioError::InvalidFrameLength => FrameDriverError::InvalidFrameLength,
        edgerun_virtio::VirtioError::NoTxDescriptor => FrameDriverError::NoTxDescriptor,
        edgerun_virtio::VirtioError::NotInitialized => FrameDriverError::NotInitialized,
        _ => FrameDriverError::Device,
    }
}

#[cfg(feature = "rtl8125")]
impl FrameDevice for rtl8125::Rtl8125 {
    fn mac(&self) -> [u8; 6] {
        self.get_mac()
    }

    fn mtu(&self) -> u16 {
        self.mtu()
    }

    fn name(&self) -> &'static str {
        "rtl8125"
    }

    fn kind(&self) -> FrameDriverKind {
        FrameDriverKind::Rtl8125
    }

    fn stats(&mut self) -> FrameDriverStats {
        let stats = self.stats();
        FrameDriverStats {
            tx_frames: stats.tx_submitted as u64,
            tx_dropped: stats.tx_dropped as u64,
            rx_frames: stats.rx_received as u64,
            rx_dropped: stats.rx_dropped as u64,
            rx_errors: stats.rx_errors as u64,
        }
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        self.send(frame)
            .then_some(())
            .ok_or(FrameDriverError::NoTxDescriptor)
    }

    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        Ok(self.recv(out))
    }
}

#[cfg(feature = "std")]
pub struct StdUdpFrameDevice {
    socket: std::net::UdpSocket,
    peer: Option<std::net::SocketAddr>,
    mac: [u8; 6],
    mtu: u16,
    stats: FrameDriverStats,
}

#[cfg(feature = "std")]
impl StdUdpFrameDevice {
    pub fn bind(addr: std::net::SocketAddr, mac: [u8; 6], mtu: u16) -> std::io::Result<Self> {
        let socket = std::net::UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            peer: None,
            mac,
            mtu,
            stats: FrameDriverStats::default(),
        })
    }

    pub fn connect(
        bind_addr: std::net::SocketAddr,
        peer: std::net::SocketAddr,
        mac: [u8; 6],
        mtu: u16,
    ) -> std::io::Result<Self> {
        let mut device = Self::bind(bind_addr, mac, mtu)?;
        device.socket.connect(peer)?;
        device.peer = Some(peer);
        Ok(device)
    }

    pub fn set_peer(&mut self, peer: std::net::SocketAddr) {
        self.peer = Some(peer);
    }

    pub fn socket(&self) -> &std::net::UdpSocket {
        &self.socket
    }
}

#[cfg(feature = "std")]
impl FrameDevice for StdUdpFrameDevice {
    fn mac(&self) -> [u8; 6] {
        self.mac
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn name(&self) -> &'static str {
        "std-udp-frame-device"
    }

    fn kind(&self) -> FrameDriverKind {
        FrameDriverKind::StdUdp
    }

    fn stats(&mut self) -> FrameDriverStats {
        self.stats
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        let Some(peer) = self.peer else {
            self.stats.tx_dropped = self.stats.tx_dropped.wrapping_add(1);
            return Err(FrameDriverError::NotInitialized);
        };
        self.socket
            .send_to(frame, peer)
            .map(|_| {
                self.stats.tx_frames = self.stats.tx_frames.wrapping_add(1);
            })
            .map_err(std_io_error)
    }

    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        match self.socket.recv_from(out) {
            Ok((len, peer)) => {
                self.peer.get_or_insert(peer);
                self.stats.rx_frames = self.stats.rx_frames.wrapping_add(1);
                Ok(Some(len))
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(None)
            }
            Err(error) => {
                self.stats.rx_errors = self.stats.rx_errors.wrapping_add(1);
                Err(std_io_error(error))
            }
        }
    }
}

#[cfg(all(feature = "tap", target_os = "linux"))]
pub struct LinuxTapFrameDevice {
    file: std::fs::File,
    name: std::string::String,
    mac: [u8; 6],
    mtu: u16,
    stats: FrameDriverStats,
}

#[cfg(all(feature = "tap", target_os = "linux"))]
impl LinuxTapFrameDevice {
    pub fn open(name: &str, mac: [u8; 6], mtu: u16) -> std::io::Result<Self> {
        use core::ffi::{c_char, c_int, c_short, c_ulong, c_void};
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;

        const IFNAMSIZ: usize = 16;
        const IFF_TAP: c_short = 0x0002;
        const IFF_NO_PI: c_short = 0x1000;
        const O_NONBLOCK: c_int = 0o4000;
        const TUNSETIFF: c_ulong = 0x4004_54ca;

        #[repr(C)]
        struct IfReq {
            name: [c_char; IFNAMSIZ],
            flags: c_short,
        }

        unsafe extern "C" {
            fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
        }

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open("/dev/net/tun")?;
        let mut request = IfReq {
            name: [0; IFNAMSIZ],
            flags: IFF_TAP | IFF_NO_PI,
        };
        for (dst, src) in request.name.iter_mut().zip(name.as_bytes().iter().copied()) {
            *dst = src as c_char;
        }
        let rc = unsafe {
            ioctl(
                file.as_raw_fd(),
                TUNSETIFF,
                &request as *const IfReq as *const c_void,
            )
        };
        if rc < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self {
            file,
            name: name.into(),
            mac,
            mtu,
            stats: FrameDriverStats::default(),
        })
    }

    pub fn interface_name(&self) -> &str {
        &self.name
    }
}

#[cfg(all(feature = "tap", target_os = "linux"))]
impl FrameDevice for LinuxTapFrameDevice {
    fn mac(&self) -> [u8; 6] {
        self.mac
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn name(&self) -> &'static str {
        "linux-tap"
    }

    fn kind(&self) -> FrameDriverKind {
        FrameDriverKind::Tap
    }

    fn stats(&mut self) -> FrameDriverStats {
        self.stats
    }

    fn try_send_frame(&mut self, frame: &[u8]) -> FrameDriverResult<()> {
        use std::io::Write as _;

        self.file
            .write_all(frame)
            .map(|_| {
                self.stats.tx_frames = self.stats.tx_frames.wrapping_add(1);
            })
            .map_err(std_io_error)
    }

    fn try_recv_frame(&mut self, out: &mut [u8]) -> FrameDriverResult<Option<usize>> {
        use std::io::Read as _;

        match self.file.read(out) {
            Ok(len) => {
                self.stats.rx_frames = self.stats.rx_frames.wrapping_add(1);
                Ok(Some(len))
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => {
                self.stats.rx_errors = self.stats.rx_errors.wrapping_add(1);
                Err(std_io_error(error))
            }
        }
    }
}

#[cfg(feature = "std")]
fn std_io_error(error: std::io::Error) -> FrameDriverError {
    match error.kind() {
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
            FrameDriverError::WouldBlock
        }
        std::io::ErrorKind::InvalidInput => FrameDriverError::InvalidFrameLength,
        std::io::ErrorKind::UnexpectedEof => FrameDriverError::InvalidBufferLength,
        _ => FrameDriverError::Device,
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameDevice, InMemoryFrameDevice};

    #[test]
    fn in_memory_device_moves_rx_and_tx_frames() {
        let mut device = InMemoryFrameDevice::<2, 2, 64>::new([1, 2, 3, 4, 5, 6], 1500);
        device.push_rx_frame(b"rx").unwrap();

        let mut buf = [0u8; 64];
        assert_eq!(device.try_recv_frame(&mut buf).unwrap(), Some(2));
        assert_eq!(&buf[..2], b"rx");

        device.try_send_frame(b"tx").unwrap();
        assert_eq!(device.pop_tx_frame(&mut buf).unwrap(), Some(2));
        assert_eq!(&buf[..2], b"tx");
    }
}
