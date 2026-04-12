//! DHCPv4 client — implements the DORA (Discover-Offer-Request-Ack) process.

use std::ffi::c_void;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::os::raw::c_int;
use std::time::{Duration, Instant};

use super::message::{DhcpMessage, DhcpMessageType, DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
use super::lease::Lease;

// ---------------------------------------------------------------------------
// Socket options (for SO_BROADCAST, SO_BINDTODEVICE)
// ---------------------------------------------------------------------------

const SOL_SOCKET: c_int = 1;
const SO_BROADCAST: c_int = 6;
const SO_BINDTODEVICE: c_int = 25;

unsafe extern "C" {
    fn setsockopt(
        fd: c_int,
        level: c_int,
        optname: c_int,
        optval: *const c_void,
        optlen: u32,
    ) -> c_int;
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// DHCPv4 client state machine.
///
/// # Example (requires root privileges and a real network interface)
/// ```ignore
/// use edgerun_dns::dhcp::DhcpClient;
///
/// let mut client = DhcpClient::new("eth0").unwrap();
/// match client.acquire_lease(std::time::Duration::from_secs(10)) {
///     Ok(lease) => println!("Got IP: {}", lease.ip),
///     Err(e) => eprintln!("DHCP failed: {}", e),
/// }
/// ```
pub struct DhcpClient {
    socket: UdpSocket,
    interface: String,
    mac: [u8; 6],
    xid: u32,
    /// Current lease if we have one.
    pub current_lease: Option<Lease>,
    /// Server ID from the last offer/ack.
    pub server_id: Option<Ipv4Addr>,
    /// Offered IP from DISCOVER.
    pub offered_ip: Option<Ipv4Addr>,
    /// When the current address acquisition started (for secs field, RFC 2131 §4.1).
    secs_start: Option<Instant>,
}

impl DhcpClient {
    /// Create a new DHCP client bound to the given interface.
    pub fn new(interface: &str) -> Result<Self, io::Error> {
        // Read MAC address from sysfs
        let mac = read_mac_from_interface(interface)?;

        // Create UDP socket bound to client port (68)
        let socket = UdpSocket::bind(("0.0.0.0", DHCP_CLIENT_PORT))?;
        socket.set_broadcast(true)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        // Bind to interface (Linux SO_BINDTODEVICE)
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;
            let fd = socket.as_raw_fd();
            let device_name = interface.as_bytes();
            let mut ifr_name = [0i8; 16];
            for (i, &b) in device_name.iter().enumerate().take(15) {
                ifr_name[i] = b as i8;
            }
            let rc = unsafe {
                setsockopt(
                    fd,
                    SOL_SOCKET,
                    SO_BINDTODEVICE,
                    ifr_name.as_ptr() as *const c_void,
                    ifr_name.len() as u32,
                )
            };
            if rc < 0 {
                // Non-fatal — may work without it
                eprintln!("edgerun-dhcp: warning: SO_BINDTODEVICE failed for {}", interface);
            }
        }

        // Random XID
        let xid = random_xid();

        Ok(Self {
            socket,
            interface: interface.to_string(),
            mac,
            xid,
            current_lease: None,
            server_id: None,
            offered_ip: None,
            secs_start: None,
        })
    }

    /// Acquire a new DHCP lease. Runs the full DORA process.
    ///
    /// Returns the lease or an error.
    pub fn acquire_lease(&mut self, timeout: Duration) -> Result<Lease, io::Error> {
        let deadline = Instant::now() + timeout;
        self.secs_start = Some(Instant::now());

        // Step 1: DISCOVER
        self.send_discover()?;

        // Step 2: Wait for OFFER
        let (offered_ip, server_id) = self.wait_for_offer(&deadline)?;

        // Step 3: REQUEST
        self.send_request(offered_ip, server_id)?;

        // Step 4: Wait for ACK
        let lease = self.wait_for_ack(&deadline)?;

        self.current_lease = Some(lease.clone());
        Ok(lease)
    }

    /// Renew the current lease.
    pub fn renew_lease(&mut self) -> Result<Lease, io::Error> {
        let lease = self.current_lease.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No active lease to renew")
        })?;

        let ip = lease.ip;
        let server_id = self.server_id.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No server ID known for renewal")
        })?;

        // Send REQUEST (unicast to server)
        let msg = DhcpMessage::request(self.xid, self.mac, ip, server_id);
        self.send_message(&msg, server_id)?;

        // Wait for ACK
        let deadline = Instant::now() + Duration::from_secs(10);
        let new_lease = self.wait_for_ack(&deadline)?;
        self.current_lease = Some(new_lease.clone());
        Ok(new_lease)
    }

    /// Release the current lease.
    pub fn release_lease(&mut self) -> Result<(), io::Error> {
        let lease = self.current_lease.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No active lease to release")
        })?;

        let server_id = self.server_id.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No server ID known")
        })?;

        let msg = DhcpMessage::release(self.xid, self.mac, lease.ip, server_id);
        // Send to server (unicast)
        let addr = SocketAddr::new(std::net::IpAddr::V4(server_id), DHCP_SERVER_PORT);
        let wire = msg.to_wire();
        let _ = self.socket.send_to(&wire, addr);

        Ok(())
    }

    // --- Internal methods ---

    fn secs_elapsed(&self) -> u16 {
        self.secs_start
            .map(|start| start.elapsed().as_secs().min(u16::MAX as u64) as u16)
            .unwrap_or(0)
    }

    fn send_discover(&mut self) -> Result<(), io::Error> {
        let mut msg = DhcpMessage::discover(self.xid, self.mac);
        msg.secs = self.secs_elapsed();
        let wire = msg.to_wire();
        let broadcast = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::BROADCAST),
            DHCP_SERVER_PORT,
        );
        self.socket.send_to(&wire, broadcast)?;
        Ok(())
    }

    fn send_request(&mut self, ip: Ipv4Addr, server_id: Ipv4Addr) -> Result<(), io::Error> {
        let mut msg = DhcpMessage::request(self.xid, self.mac, ip, server_id);
        msg.secs = self.secs_elapsed();
        let wire = msg.to_wire();
        let broadcast = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::BROADCAST),
            DHCP_SERVER_PORT,
        );
        self.socket.send_to(&wire, broadcast)?;
        Ok(())
    }

    fn send_message(&mut self, msg: &DhcpMessage, dest: Ipv4Addr) -> Result<(), io::Error> {
        let wire = msg.to_wire();
        let addr = SocketAddr::new(std::net::IpAddr::V4(dest), DHCP_SERVER_PORT);
        self.socket.send_to(&wire, addr)?;
        Ok(())
    }

    fn wait_for_offer(&mut self, deadline: &Instant) -> Result<(Ipv4Addr, Ipv4Addr), io::Error> {
        while Instant::now() < *deadline {
            match self.recv_message() {
                Ok((msg, _src)) => {
                    if msg.xid != self.xid {
                        continue;
                    }
                    if msg.options.message_type == Some(DhcpMessageType::Offer) {
                        let yiaddr = msg.yiaddr;
                        let server_id = msg.options.server_id.ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "OFFER missing server ID")
                        })?;
                        self.offered_ip = Some(yiaddr);
                        self.server_id = Some(server_id);
                        return Ok((yiaddr, server_id));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_) => continue,
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "No DHCP OFFER received",
        ))
    }

    fn wait_for_ack(&mut self, deadline: &Instant) -> Result<Lease, io::Error> {
        while Instant::now() < *deadline {
            match self.recv_message() {
                Ok((msg, _src)) => {
                    if msg.xid != self.xid {
                        continue;
                    }
                    match msg.options.message_type {
                        Some(DhcpMessageType::Ack) => {
                            let lease = Lease::new(
                                self.mac,
                                msg.yiaddr,
                                msg.options.lease_time.unwrap_or(3600),
                                self.xid,
                            );
                            self.server_id = msg.options.server_id;
                            return Ok(lease);
                        }
                        Some(DhcpMessageType::Nak) => {
                            return Err(io::Error::new(
                                io::ErrorKind::ConnectionRefused,
                                "DHCP server sent NAK",
                            ));
                        }
                        _ => continue,
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_) => continue,
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "No DHCP ACK/NAK received",
        ))
    }

    fn recv_message(&mut self) -> Result<(DhcpMessage, Ipv4Addr), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = self.socket.recv_from(&mut buf)?;
        let msg = DhcpMessage::from_wire(&buf[..n])?;
        let src_ip = match src.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => Ipv4Addr::UNSPECIFIED,
        };
        Ok((msg, src_ip))
    }

    /// Return the MAC address of this client.
    pub fn mac(&self) -> [u8; 6] {
        self.mac
    }

    /// Return the current XID.
    pub fn xid(&self) -> u32 {
        self.xid
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn read_mac_from_interface(interface: &str) -> Result<[u8; 6], io::Error> {
    // Read from sysfs
    let path = format!("/sys/class/net/{}/address", interface);
    let mac_str = std::fs::read_to_string(&path)?;
    let mac_str = mac_str.trim();

    let parts: Vec<u8> = mac_str
        .split(':')
        .map(|s| u8::from_str_radix(s, 16).unwrap_or(0))
        .collect();

    if parts.len() != 6 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid MAC address: {}", mac_str),
        ));
    }

    let mut mac = [0u8; 6];
    mac.copy_from_slice(&parts);
    Ok(mac)
}

fn random_xid() -> u32 {
    // Simple random XID from time-based
    use std::time::SystemTime;
    let d = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    (d.as_millis() as u32).wrapping_mul(2654435761) // Knuth multiplicative
}
