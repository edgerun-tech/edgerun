#![allow(dead_code)]
#![allow(unused_must_use)]

use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_linux_netif::{
    discover_network_interfaces, discover_network_interfaces_in, LinuxNetifBackend,
};
use edgerun_linux_sysfs::{
    close_ioctl_fd, fill_ifr_name, ioctl_call, open_ioctl_socket, read_trimmed,
};
use edgerun_network_interface::{
    NetworkAdminState, NetworkInterfaceController, NetworkInterfaceKind,
};
use edgerun_wifi::{
    default_wifi_descriptor, validate_access_point_config, WifiAccessPointConfig,
    WifiAccessPointController, WifiAccessPointState, WifiController, WifiInterfaceInfo,
    WifiInterfaceMode, WifiNetworkObservation, WifiPowerState, WifiScanResult, WifiScanner,
};
use std::fs;
use std::io;
use std::mem;
use std::os::raw::{c_char, c_ulong};
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// nl80211 netlink constants (kernel WiFi configuration)
// ============================================================================

const NETLINK_GENERIC: i32 = 16;
const NLM_F_REQUEST: u16 = 1;
const NLM_F_ACK: u16 = 4;
const NLM_F_DUMP: u16 = 0x300;

const GENL_ID_CTRL: u8 = 16;
const CTRL_CMD_GETFAMILY: u8 = 3;

const NL80211_GENL_NAME: &[u8] = b"nl80211";

// nl80211 commands
const NL80211_CMD_GET_WIPHY: u8 = 1;
const NL80211_CMD_SET_INTERFACE: u8 = 5;
const NL80211_CMD_NEW_SCAN_RESULTS: u8 = 21;
const NL80211_CMD_SCAN: u8 = 30;
const NL80211_CMD_REG_CHANGE: u8 = 34;
const NL80211_CMD_CONNECT: u8 = 54;
const NL80211_CMD_DISCONNECT: u8 = 55;
const NL80211_CMD_GET_SCAN: u8 = 56;
const NL80211_CMD_TRIGGER_SCAN: u8 = 57;
const NL80211_CMD_REGISTER_FRAME: u8 = 79;
const NL80211_CMD_FRAME: u8 = 80;
const NL80211_CMD_FRAME_WAIT_CANCEL: u8 = 81;
const NL80211_CMD_SET_BSS: u8 = 87;
const NL80211_CMD_GET_INTERFACE: u8 = 7;
const NL80211_CMD_GET_STATION: u8 = 58;
const NL80211_CMD_NEW_INTERFACE: u8 = 13;
const NL80211_CMD_DEL_INTERFACE: u8 = 14;
const NL80211_CMD_START_AP: u8 = 52;
const NL80211_CMD_STOP_AP: u8 = 53;
const NL80211_CMD_SET_WIPHY: u8 = 9;
const NL80211_CMD_GET_REG: u8 = 36;
const NL80211_CMD_SET_REG: u8 = 37;
const NL80211_CMD_REQ_SET_REG: u8 = 38;

// nl80211 attributes (top-level)
const NL80211_ATTR_IFINDEX: u16 = 3;
const NL80211_ATTR_SSID: u16 = 4;
const NL80211_ATTR_BSSID: u16 = 6;
const NL80211_ATTR_IFTYPE: u16 = 7;
const NL80211_ATTR_SCAN_FREQUENCIES: u16 = 10;
const NL80211_ATTR_SCAN_SSIDS: u16 = 11;
const NL80211_ATTR_GENERATION: u16 = 12;
const NL80211_ATTR_WIPHY: u16 = 13;
const NL80211_ATTR_WIPHY_FREQ: u16 = 14;
const NL80211_ATTR_COOKIE: u16 = 18;
const NL80211_ATTR_STATUS_CODE: u16 = 24;
const NL80211_ATTR_WIPHY_NAME: u16 = 27;
const NL80211_ATTR_CONTROL_PORT: u16 = 36;
const NL80211_ATTR_MAC: u16 = 42;
const NL80211_ATTR_WIPHY_RETRY_SHORT: u16 = 47;
const NL80211_ATTR_WIPHY_RETRY_LONG: u16 = 48;
const NL80211_ATTR_WIPHY_FRAG_THRESHOLD: u16 = 49;
const NL80211_ATTR_WIPHY_RTS_THRESHOLD: u16 = 50;
const NL80211_ATTR_KEYS: u16 = 51;
const NL80211_ATTR_CIPHER_SUITES_PAIRWISE: u16 = 53;
const NL80211_ATTR_CIPHER_SUITE_GROUP: u16 = 54;
const NL80211_ATTR_KEY_DATA: u16 = 63;
const NL80211_ATTR_KEY_IDX: u16 = 62;
const NL80211_ATTR_IE: u16 = 65;
const NL80211_ATTR_MAX_NUM_SCAN_SSIDS: u16 = 74;
const NL80211_ATTR_SCAN_FLAGS: u16 = 85;
const NL80211_ATTR_WIPHY_BANDS: u16 = 90;
const NL80211_ATTR_KEY_SEQ: u16 = 92;
const NL80211_ATTR_KEY_DEFAULT: u16 = 104;
const NL80211_ATTR_PMK: u16 = 167;
const NL80211_ATTR_AUTH_TYPE: u16 = 67;
const NL80211_ATTR_BEACON_INTERVAL: u16 = 76;
const NL80211_ATTR_DTIM_PERIOD: u16 = 77;
const NL80211_ATTR_BEACON_HEAD: u16 = 150;
const NL80211_ATTR_BEACON_TAIL: u16 = 151;
const NL80211_ATTR_REG_ALPHA2: u16 = 17;
const NL80211_ATTR_REG_RULES: u16 = 180;
const NL80211_ATTR_REG_INITIATOR: u16 = 108;
const NL80211_ATTR_REG_TYPE: u16 = 107;
const NL80211_ATTR_WIPHY_COVERAGE_CLASS: u16 = 311;

// nl80211 BSS attributes (nested in GET_SCAN responses)
const NL80211_BSS_INFORMATION_ELEMENTS: u16 = 1;
const NL80211_BSS_BSSID: u16 = 2;
const NL80211_BSS_FREQUENCY: u16 = 3;
const NL80211_BSS_SIGNAL_MBM: u16 = 10;
const NL80211_BSS_SIGNAL_UNSPEC: u16 = 11;

// nl80211 station attributes (nested in GET_STATION responses)
const NL80211_STA_INFO_SIGNAL: u16 = 9;
const NL80211_STA_INFO_TX_BITRATE: u16 = 2;

// nl80211 interface types
const NL80211_IFTYPE_STATION: u32 = 2;
const NL80211_IFTYPE_AP: u32 = 3;

// Regulatory domain types (NL80211_REGDOM_*)
const NL80211_REGDOM_SET_BY_CORE: u8 = 0;
const NL80211_REGDOM_SET_BY_USER: u8 = 1;
const NL80211_REGDOM_SET_BY_DRIVER: u8 = 2;
const NL80211_REGDOM_SET_BY_COUNTRY_IE: u8 = 3;
const NL80211_REGDOM_TYPE_COUNTRY: u8 = 0;
const NL80211_REGDOM_TYPE_WORLD: u8 = 1;
const NL80211_REGDOM_TYPE_INTERSECTION: u8 = 2;
const NL80211_REGDOM_TYPE_STRICT_WORLD: u8 = 3;

// nl80211 authentication types
const NL80211_AUTHTYPE_OPEN_SYSTEM: u32 = 1;
const NL80211_AUTHTYPE_FT: u32 = 4;

// nl80211 scan flags
const NL80211_SCAN_FLAG_FLUSH: u32 = 1 << 1;

// Cipher suites (from IEEE 802.11 OUI 00-0F-AC)
const WIFI_CIPHER_SUITE_CCMP: u32 = 0x000FAC04; // CCMP (AES) - WPA2
const WIFI_CIPHER_SUITE_TKIP: u32 = 0x000FAC02; // TKIP - WPA
const WIFI_CIPHER_SUITE_WEP40: u32 = 0x000FAC01;
const WIFI_CIPHER_SUITE_WEP104: u32 = 0x000FAC05;
const WIFI_CIPHER_SUITE_BIP: u32 = 0x000FAC06;
const WIFI_CIPHER_SUITE_GCMP: u32 = 0x000FAC08;
const WIFI_CIPHER_SUITE_CCMP_256: u32 = 0x000FAC0A;
const WIFI_CIPHER_SUITE_GMAC: u32 = 0x000FAC0B;
const WIFI_CIPHER_SUITE_GMAC_256: u32 = 0x000FAC0C;

// Generic netlink attribute types
const NLA_F_NESTED: u16 = 1 << 15;

// RSN IE OUI
const RSN_OUI: [u8; 3] = [0x00, 0x0F, 0xAC];

// WPA IE OUI
const WPA_OUI: [u8; 4] = [0x00, 0x50, 0xF2, 0x01];

// ============================================================================
// nl80211 netlink socket implementation
// ============================================================================

/// Netlink socket for generic netlink communication with nl80211.
struct Nl80211Socket {
    fd: RawFd,
    family_id: u16,
    seq: u32,
    port_id: u32,
}

#[repr(C)]
struct NlMsghdr {
    nlmsg_len: u32,
    nlmsg_type: u16,
    nlmsg_flags: u16,
    nlmsg_seq: u32,
    nlmsg_pid: u32,
}

#[repr(C)]
struct GenlMsghdr {
    cmd: u8,
    version: u8,
    reserved: u16,
}

#[repr(C)]
struct NlAttr {
    nla_len: u16,
    nla_type: u16,
}

#[repr(C)]
struct SockaddrNl {
    nl_family: u16,
    nl_pad: u16,
    nl_pid: u32,
    nl_groups: u32,
}

impl Nl80211Socket {
    /// Open a generic netlink socket and resolve the nl80211 family ID.
    fn open() -> Result<Self, CapabilityError> {
        let fd = unsafe {
            libc::socket(libc::AF_NETLINK, libc::SOCK_RAW, NETLINK_GENERIC)
        };
        if fd < 0 {
            return Err(CapabilityError::Provider(
                format!("failed to create netlink socket: {}", io::Error::last_os_error()).into(),
            ));
        }

        // Bind to a local port
        let mut addr: SockaddrNl = unsafe { mem::zeroed() };
        addr.nl_family = libc::AF_NETLINK as u16;
        addr.nl_pid = unsafe { libc::getpid() } as u32;
        if unsafe { libc::bind(fd, &addr as *const _ as *const _, mem::size_of::<SockaddrNl>() as u32) } < 0 {
            unsafe { libc::close(fd) };
            return Err(CapabilityError::Provider(
                format!("failed to bind netlink socket: {}", io::Error::last_os_error()).into(),
            ));
        }

        let mut sock = Self {
            fd,
            family_id: 0,
            seq: 0,
            port_id: addr.nl_pid,
        };

        // Resolve nl80211 family ID
        sock.family_id = sock.resolve_family()?;
        Ok(sock)
    }

    /// Send a netlink message and wait for the ACK/response.
    fn request(&mut self, cmd: u8, _ifindex: i32, attrs: &[u8]) -> Result<Vec<u8>, CapabilityError> {
        self.seq += 1;

        // Build netlink header
        let mut msg = Vec::new();
        let hdr = NlMsghdr {
            nlmsg_len: 0, // filled in later
            nlmsg_type: self.family_id as u16,
            nlmsg_flags: NLM_F_REQUEST | NLM_F_ACK,
            nlmsg_seq: self.seq,
            nlmsg_pid: self.port_id,
        };
        msg.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&hdr as *const _ as *const u8, mem::size_of::<NlMsghdr>())
        });

        // Generic netlink header
        let genl = GenlMsghdr {
            cmd,
            version: 1,
            reserved: 0,
        };
        msg.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&genl as *const _ as *const u8, mem::size_of::<GenlMsghdr>())
        });

        // Append attributes
        msg.extend_from_slice(attrs);

        // Update message length
        let len = msg.len() as u32;
        msg[..4].copy_from_slice(&len.to_ne_bytes());

        // Send
        let mut dest: SockaddrNl = unsafe { mem::zeroed() };
        dest.nl_family = libc::AF_NETLINK as u16;
        let sent = unsafe {
            libc::sendto(
                self.fd,
                msg.as_ptr() as *const _,
                msg.len(),
                0,
                &dest as *const _ as *const _,
                mem::size_of::<SockaddrNl>() as u32,
            )
        };
        if sent < 0 {
            return Err(CapabilityError::Provider(
                format!("netlink send failed: {}", io::Error::last_os_error()).into(),
            ));
        }

        // Receive response(s)
        self.recv_responses()
    }

    /// Send a dump request (returns multiple messages).
    fn request_dump(&mut self, cmd: u8, _ifindex: i32, attrs: &[u8]) -> Result<Vec<u8>, CapabilityError> {
        self.seq += 1;

        let mut msg = Vec::new();
        let hdr = NlMsghdr {
            nlmsg_len: 0,
            nlmsg_type: self.family_id as u16,
            nlmsg_flags: NLM_F_REQUEST | NLM_F_DUMP,
            nlmsg_seq: self.seq,
            nlmsg_pid: self.port_id,
        };
        msg.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&hdr as *const _ as *const u8, mem::size_of::<NlMsghdr>())
        });

        let genl = GenlMsghdr {
            cmd,
            version: 1,
            reserved: 0,
        };
        msg.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&genl as *const _ as *const u8, mem::size_of::<GenlMsghdr>())
        });

        msg.extend_from_slice(attrs);

        let len = msg.len() as u32;
        msg[..4].copy_from_slice(&len.to_ne_bytes());

        let mut dest: SockaddrNl = unsafe { mem::zeroed() };
        dest.nl_family = libc::AF_NETLINK as u16;
        let sent = unsafe {
            libc::sendto(
                self.fd,
                msg.as_ptr() as *const _,
                msg.len(),
                0,
                &dest as *const _ as *const _,
                mem::size_of::<SockaddrNl>() as u32,
            )
        };
        if sent < 0 {
            return Err(CapabilityError::Provider(
                format!("netlink send failed: {}", io::Error::last_os_error()).into(),
            ));
        }

        self.recv_responses()
    }

    /// Receive netlink responses until DONE or ERROR.
    fn recv_responses(&mut self) -> Result<Vec<u8>, CapabilityError> {
        let mut all_data = Vec::new();
        let mut buf = vec![0u8; 8192];

        loop {
            let mut src: SockaddrNl = unsafe { mem::zeroed() };
            let mut src_len: u32 = mem::size_of::<SockaddrNl>() as u32;
            let received = unsafe {
                libc::recvfrom(
                    self.fd,
                    buf.as_mut_ptr() as *mut _,
                    buf.len(),
                    0,
                    &mut src as *mut _ as *mut _,
                    &mut src_len,
                )
            };
            if received < 0 {
                return Err(CapabilityError::Provider(
                    format!("netlink recv failed: {}", io::Error::last_os_error()).into(),
                ));
            }

            let data = &buf[..received as usize];
            all_data.extend_from_slice(data);

            // Check if this is the last message (NLMSG_DONE or NLMSG_ERROR)
            if data.len() < mem::size_of::<NlMsghdr>() {
                continue;
            }
            let hdr: NlMsghdr = unsafe {
                std::ptr::read_unaligned(data.as_ptr() as *const NlMsghdr)
            };

            // NLMSG_ERROR with error=0 is ACK (success)
            if hdr.nlmsg_type == libc::NLMSG_ERROR as u16 {
                if data.len() >= mem::size_of::<NlMsghdr>() + 4 {
                    let error: i32 = unsafe {
                        std::ptr::read_unaligned(data[mem::size_of::<NlMsghdr>()..].as_ptr() as *const i32)
                    };
                    if error != 0 {
                        return Err(CapabilityError::Provider(
                            format!("nl80211 error: {}", io::Error::from_raw_os_error(-error)).into(),
                        ));
                    }
                }
                break; // Done (ACK with error=0)
            }

            // NLMSG_DONE
            if hdr.nlmsg_type == libc::NLMSG_DONE as u16 {
                break;
            }

            // Check for multipart flag
            if hdr.nlmsg_flags & (libc::NLM_F_MULTI as u16) == 0 {
                break; // Single message, done
            }
        }

        Ok(all_data)
    }

    /// Resolve the nl80211 generic netlink family ID.
    fn resolve_family(&mut self) -> Result<u16, CapabilityError> {
        // Build CTRL_CMD_GETFAMILY request
        let mut attrs = Vec::new();
        put_nla_string(&mut attrs, 1, NL80211_GENL_NAME); // CTRL_ATTR_FAMILY_NAME

        self.seq += 1;
        let mut msg = Vec::new();
        let hdr = NlMsghdr {
            nlmsg_len: 0,
            nlmsg_type: GENL_ID_CTRL as u16,
            nlmsg_flags: NLM_F_REQUEST | NLM_F_ACK,
            nlmsg_seq: self.seq,
            nlmsg_pid: self.port_id,
        };
        msg.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&hdr as *const _ as *const u8, mem::size_of::<NlMsghdr>())
        });
        let genl = GenlMsghdr {
            cmd: CTRL_CMD_GETFAMILY,
            version: 1,
            reserved: 0,
        };
        msg.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&genl as *const _ as *const u8, mem::size_of::<GenlMsghdr>())
        });
        msg.extend_from_slice(&attrs);
        let len = msg.len() as u32;
        msg[..4].copy_from_slice(&len.to_ne_bytes());

        let mut dest: SockaddrNl = unsafe { mem::zeroed() };
        dest.nl_family = libc::AF_NETLINK as u16;
        let sent = unsafe {
            libc::sendto(
                self.fd,
                msg.as_ptr() as *const _,
                msg.len(),
                0,
                &dest as *const _ as *const _,
                mem::size_of::<SockaddrNl>() as u32,
            )
        };
        if sent < 0 {
            return Err(CapabilityError::Provider(
                format!("netlink send failed: {}", io::Error::last_os_error()).into(),
            ));
        }

        // Parse response to find family ID
        let response = self.recv_responses()?;
        parse_ctrl_getfamily_response(&response)
    }
}

impl Drop for Nl80211Socket {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

// ============================================================================
// Netlink attribute helpers
// ============================================================================

fn put_nla(buf: &mut Vec<u8>, attr_type: u16, data: &[u8]) {
    let len = (mem::size_of::<NlAttr>() + data.len() + 3) & !3; // NLA_ALIGN
    buf.resize(buf.len() + len, 0);
    let start = buf.len() - len;
    let attr = NlAttr {
        nla_len: (mem::size_of::<NlAttr>() + data.len()) as u16,
        nla_type: attr_type,
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &attr as *const _ as *const u8,
            buf[start..].as_mut_ptr(),
            mem::size_of::<NlAttr>(),
        );
        std::ptr::copy_nonoverlapping(
            data.as_ptr(),
            buf[start + mem::size_of::<NlAttr>()..].as_mut_ptr(),
            data.len(),
        );
    }
}

fn put_nla_u32(buf: &mut Vec<u8>, attr_type: u16, value: u32) {
    put_nla(buf, attr_type, &value.to_ne_bytes());
}

fn put_nla_string(buf: &mut Vec<u8>, attr_type: u16, s: &[u8]) {
    put_nla(buf, attr_type, s);
}

fn put_nla_u16(buf: &mut Vec<u8>, attr_type: u16, value: u16) {
    put_nla(buf, attr_type, &value.to_ne_bytes());
}

fn put_nla_u8(buf: &mut Vec<u8>, attr_type: u16, value: u8) {
    put_nla(buf, attr_type, &[value]);
}

fn put_nla_nested(buf: &mut Vec<u8>, attr_type: u16, inner: &[u8]) {
    // Nested attribute: outer header + inner data
    let total_len = mem::size_of::<NlAttr>() + inner.len();
    let aligned_len = (total_len + 3) & !3;
    buf.resize(buf.len() + aligned_len, 0);
    let start = buf.len() - aligned_len;
    let attr = NlAttr {
        nla_len: total_len as u16,
        nla_type: attr_type | NLA_F_NESTED,
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &attr as *const _ as *const u8,
            buf[start..].as_mut_ptr(),
            mem::size_of::<NlAttr>(),
        );
        std::ptr::copy_nonoverlapping(
            inner.as_ptr(),
            buf[start + mem::size_of::<NlAttr>()..].as_mut_ptr(),
            inner.len(),
        );
    }
}

/// Parse a CTRL_CMD_GETFAMILY response to extract the nl80211 family ID.
fn parse_ctrl_getfamily_response(data: &[u8]) -> Result<u16, CapabilityError> {
    if data.len() < mem::size_of::<NlMsghdr>() + mem::size_of::<GenlMsghdr>() {
        return Err(CapabilityError::Provider("short nl80211 family response".into()));
    }

    // Skip netlink header, then generic netlink header
    let genl_start = mem::size_of::<NlMsghdr>();
    let attr_start = genl_start + mem::size_of::<GenlMsghdr>();

    // Parse attributes looking for CTRL_ATTR_FAMILY_ID (type 2)
    let mut offset = attr_start;
    while offset + mem::size_of::<NlAttr>() <= data.len() {
        let attr: NlAttr = unsafe {
            std::ptr::read_unaligned(data[offset..].as_ptr() as *const NlAttr)
        };
        let data_start = offset + mem::size_of::<NlAttr>();
        let data_end = data_start + (attr.nla_len as usize - mem::size_of::<NlAttr>());

        if attr.nla_type == 2 {
            // CTRL_ATTR_FAMILY_ID
            if data_end <= data.len() && (data_end - data_start) >= 2 {
                let id = u16::from_ne_bytes([data[data_start], data[data_start + 1]]);
                return Ok(id);
            }
        }

        // Align to next attribute
        offset = (offset + (attr.nla_len as usize) + 3) & !3;
    }

    Err(CapabilityError::Provider("nl80211 family ID not found in response".into()))
}

// ============================================================================
// nl80211 scan implementation
// ============================================================================

/// Trigger an nl80211 scan and return the results.
///
/// If `flush` is true, cached scan results are cleared before scanning.
/// If `frequencies` is provided, only those frequencies (MHz) are scanned.
fn nl80211_scan(ifindex: i32, ssids: Option<&[&[u8]]>, flush: bool, frequencies: Option<&[u32]>) -> Result<Vec<WifiNetworkObservation>, CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    // Build TRIGGER_SCAN attributes
    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);
    if let Some(ssid_list) = ssids {
        // Build nested SSID attribute
        let mut ssid_attrs = Vec::new();
        for (i, ssid) in ssid_list.iter().enumerate() {
            put_nla_string(&mut ssid_attrs, i as u16, ssid);
        }
        put_nla_nested(&mut attrs, NL80211_ATTR_SCAN_SSIDS, &ssid_attrs);
    } else {
        // Wildcard scan: one empty SSID
        let mut ssid_attrs = Vec::new();
        put_nla_string(&mut ssid_attrs, 0, b"");
        put_nla_nested(&mut attrs, NL80211_ATTR_SCAN_SSIDS, &ssid_attrs);
    }

    // Scan specific frequencies if provided
    if let Some(freqs) = frequencies {
        let mut freq_data = Vec::new();
        for &freq in freqs {
            freq_data.extend_from_slice(&freq.to_ne_bytes());
        }
        put_nla(&mut attrs, NL80211_ATTR_SCAN_FREQUENCIES, &freq_data);
    }

    // Flush cached results if requested
    if flush {
        put_nla_u32(&mut attrs, NL80211_ATTR_SCAN_FLAGS, NL80211_SCAN_FLAG_FLUSH);
    }

    // Trigger the scan
    sock.request(NL80211_CMD_TRIGGER_SCAN, ifindex, &attrs)?;

    // Wait a bit for scan to complete
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Get scan results via dump
    let mut get_attrs = Vec::new();
    put_nla_u32(&mut get_attrs, NL80211_ATTR_IFINDEX, ifindex as u32);

    let response = sock.request_dump(NL80211_CMD_GET_SCAN, ifindex, &get_attrs)?;

    parse_scan_results(&response)
}

/// Parse nl80211 scan results from netlink response data.
fn parse_scan_results(data: &[u8]) -> Result<Vec<WifiNetworkObservation>, CapabilityError> {
    let mut observations = Vec::new();
    let mut offset = 0;

    while offset + mem::size_of::<NlMsghdr>() <= data.len() {
        let hdr: NlMsghdr = unsafe {
            std::ptr::read_unaligned(data[offset..].as_ptr() as *const NlMsghdr)
        };

        // Stop at NLMSG_DONE or NLMSG_ERROR
        if hdr.nlmsg_type == libc::NLMSG_DONE as u16 || hdr.nlmsg_type == libc::NLMSG_ERROR as u16 {
            break;
        }

        // Only process NLMSG_ERROR (which is the scan result in our case since we used NLM_F_ACK)
        // Actually for dump, results come as NLMSG_ERROR with cmd = NL80211_CMD_GET_SCAN
        // Let's parse more carefully

        let msg_end = offset + hdr.nlmsg_len as usize;
        if msg_end > data.len() {
            break;
        }

        let genl_start = offset + mem::size_of::<NlMsghdr>();
        if genl_start + mem::size_of::<GenlMsghdr>() > msg_end {
            offset = (offset + hdr.nlmsg_len as usize + 3) & !3;
            continue;
        }

        let genl: GenlMsghdr = unsafe {
            std::ptr::read_unaligned(data[genl_start..].as_ptr() as *const GenlMsghdr)
        };

        // Only process NEW_SCAN_RESULTS or GET_SCAN responses
        if genl.cmd == NL80211_CMD_NEW_SCAN_RESULTS || genl.cmd == NL80211_CMD_GET_SCAN {
            let attr_start = genl_start + mem::size_of::<GenlMsghdr>();
            if let Some(obs) = parse_bss_info(data, attr_start, msg_end) {
                observations.push(obs);
            }
        }

        offset = (msg_end + 3) & !3;
    }

    Ok(observations)
}

/// Parse a single BSS info entry from nl80211 attributes.
fn parse_bss_info(data: &[u8], attr_start: usize, msg_end: usize) -> Option<WifiNetworkObservation> {
    let mut ssid: Option<String> = None;
    let mut bssid: Option<String> = None;
    let mut signal_mbm: Option<i32> = None; // signal in mBm (milli-Bel-milli)
    let mut frequency: Option<u32> = None;
    let mut seen_rsn = false;

    let mut offset = attr_start;
    while offset + mem::size_of::<NlAttr>() <= msg_end {
        let attr: NlAttr = unsafe {
            std::ptr::read_unaligned(data[offset..].as_ptr() as *const NlAttr)
        };
        let data_start = offset + mem::size_of::<NlAttr>();
        let data_end = data_start + (attr.nla_len as usize - mem::size_of::<NlAttr>());

        if data_end > msg_end {
            break;
        }

        let attr_type = attr.nla_type & !NLA_F_NESTED;

        match attr_type {
            1 => {
                // NL80211_BSS_INFORMATION_ELEMENTS - contains SSID IE (type 0) and RSN IE (type 48)
                let mut ie_offset = 0;
                while ie_offset + 2 <= data_end - data_start {
                    let ie_type = data[data_start + ie_offset];
                    let ie_len = data[data_start + ie_offset + 1] as usize;
                    if ie_offset + 2 + ie_len > data_end - data_start {
                        break;
                    }
                    let ie_data = &data[data_start + ie_offset + 2..data_start + ie_offset + 2 + ie_len];

                    if ie_type == 0 && ssid.is_none() {
                        // SSID IE
                        if let Ok(s) = std::str::from_utf8(ie_data) {
                            ssid = Some(s.to_string());
                        }
                    } else if ie_type == 48 {
                        // RSN IE (WPA2)
                        seen_rsn = true;
                    } else if ie_type == 221 && ie_data.len() >= 4 {
                        // Vendor-specific IE — check for WPA OUI (00-50-F2:01)
                        if ie_data[..4] == WPA_OUI {
                            seen_rsn = true; // WPA legacy treated as RSN-capable
                        }
                    }

                    ie_offset += 2 + ie_len;
                }
            }
            2 => {
                // NL80211_BSS_BSSID - 6 bytes
                if data_end - data_start >= 6 {
                    bssid = Some(format!(
                        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                        data[data_start], data[data_start + 1], data[data_start + 2],
                        data[data_start + 3], data[data_start + 4], data[data_start + 5]
                    ));
                }
            }
            3 => {
                // NL80211_BSS_FREQUENCY
                if data_end - data_start >= 4 {
                    frequency = Some(u32::from_ne_bytes([
                        data[data_start], data[data_start + 1], data[data_start + 2], data[data_start + 3]
                    ]));
                }
            }
            10 | 11 => {
                // NL80211_BSS_SIGNAL_MBM or NL80211_BSS_SIGNAL_UNSPEC
                if data_end - data_start >= 4 {
                    signal_mbm = Some(i32::from_ne_bytes([
                        data[data_start], data[data_start + 1], data[data_start + 2], data[data_start + 3]
                    ]));
                }
            }
            _ => {}
        }

        offset = (offset + attr.nla_len as usize + 3) & !3;
    }

    // Skip BSS entries without SSID (hidden networks with empty SSID)
    if ssid.as_deref().unwrap_or("").is_empty() {
        return None;
    }

    let signal_dbm = signal_mbm.map(|mbm| (mbm / 100) as i16);

    Some(WifiNetworkObservation {
        interface_name: String::new(), // filled in by caller
        ssid,
        bssid,
        signal_dbm,
        frequency_mhz: frequency,
        secure: Some(seen_rsn),
        observed_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64,
    })
}

// ============================================================================
// nl80211 connect implementation (WPA-PSK via control port)
// ============================================================================

/// Build RSN IE for WPA2-PSK with CCMP.
fn build_rsn_ie() -> Vec<u8> {
    let mut ie = Vec::new();
    ie.push(48); // RSN IE type
    ie.push(20); // Length: 20 bytes for WPA2-PSK + CCMP

    // RSN version (1)
    ie.extend_from_slice(&1u16.to_le_bytes());

    // Group cipher suite (CCMP = 00-0F-AC:4)
    ie.extend_from_slice(&RSN_OUI);
    ie.push(0x04);

    // Pairwise cipher suite count (1)
    ie.push(1);
    // Pairwise cipher suite (CCMP)
    ie.extend_from_slice(&RSN_OUI);
    ie.push(0x04);

    // AKM suite count (1)
    ie.push(1);
    // AKM suite (PSK = 00-0F-AC:2)
    ie.extend_from_slice(&RSN_OUI);
    ie.push(0x02);

    // No RSN capabilities
    ie.extend_from_slice(&0u16.to_le_bytes());

    ie
}

/// Connect to a WPA-protected network via nl80211.
///
/// If `pmk` is provided, it is passed to the kernel via NL80211_ATTR_PMK
/// and the kernel handles the 4-way handshake internally.
/// If `pmk` is None, the control port is set for userspace EAPOL handling.
fn nl80211_connect_wpa(
    ifindex: i32,
    ssid: &[u8],
    bssid: Option<&[u8; 6]>,
    frequency: Option<u32>,
    pmk: Option<&[u8; 32]>,
) -> Result<(), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);
    put_nla_string(&mut attrs, NL80211_ATTR_SSID, ssid);

    if let Some(b) = bssid {
        put_nla(&mut attrs, NL80211_ATTR_BSSID, b);
    }

    // Open system authentication
    put_nla_u32(&mut attrs, NL80211_ATTR_AUTH_TYPE, NL80211_AUTHTYPE_OPEN_SYSTEM);

    if let Some(pmk_bytes) = pmk {
        // Pass PMK to the kernel — it handles the 4-way handshake internally
        put_nla(&mut attrs, NL80211_ATTR_PMK, pmk_bytes);
    } else {
        // No PMK — set control port for userspace EAPOL handling
        put_nla(&mut attrs, NL80211_ATTR_CONTROL_PORT, &[1]);
    }

    // Add RSN IE for WPA2-PSK
    let rsn_ie = build_rsn_ie();
    let mut ie_data = Vec::new();
    ie_data.extend_from_slice(&rsn_ie);
    put_nla_string(&mut attrs, NL80211_ATTR_IE, &ie_data);

    // Set cipher suites
    let mut pairwise = Vec::new();
    pairwise.extend_from_slice(&WIFI_CIPHER_SUITE_CCMP.to_ne_bytes());
    put_nla(&mut attrs, NL80211_ATTR_CIPHER_SUITES_PAIRWISE, &pairwise);
    put_nla_u32(&mut attrs, NL80211_ATTR_CIPHER_SUITE_GROUP, WIFI_CIPHER_SUITE_CCMP);

    if let Some(freq) = frequency {
        put_nla_u32(&mut attrs, 14, freq); // NL80211_ATTR_WIPHY_FREQ
    }

    sock.request(NL80211_CMD_CONNECT, ifindex, &attrs)?;
    Ok(())
}

/// Disconnect from the current network.
fn nl80211_disconnect(ifindex: i32) -> Result<(), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);

    sock.request(NL80211_CMD_DISCONNECT, ifindex, &attrs)?;
    Ok(())
}

/// Set interface type (station/AP).
fn nl80211_set_interface_type(ifindex: i32, iftype: u32) -> Result<(), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);
    put_nla_u32(&mut attrs, NL80211_ATTR_IFTYPE, iftype);

    sock.request(NL80211_CMD_SET_INTERFACE, ifindex, &attrs)?;
    Ok(())
}

/// Query the current interface type (station/AP/ad-hoc/monitor) via nl80211.
fn nl80211_get_interface_type(ifindex: i32) -> Result<u32, CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);

    let response = sock.request(NL80211_CMD_GET_INTERFACE, ifindex, &attrs)?;

    // Parse the response to find NL80211_ATTR_IFTYPE
    let msg_end = response.len();
    let genl_start = mem::size_of::<NlMsghdr>() + mem::size_of::<GenlMsghdr>();
    if genl_start + mem::size_of::<NlAttr>() > msg_end {
        return Err(CapabilityError::Provider("truncated GET_INTERFACE response".into()));
    }

    let mut offset = genl_start;
    while offset + mem::size_of::<NlAttr>() <= msg_end {
        let attr: NlAttr = unsafe {
            std::ptr::read_unaligned(response[offset..].as_ptr() as *const NlAttr)
        };
        let data_start = offset + mem::size_of::<NlAttr>();
        let data_end = data_start + (attr.nla_len as usize - mem::size_of::<NlAttr>());

        if data_end > msg_end {
            break;
        }

        if attr.nla_type == NL80211_ATTR_IFTYPE && data_end - data_start >= 4 {
            let iftype = u32::from_ne_bytes([
                response[data_start],
                response[data_start + 1],
                response[data_start + 2],
                response[data_start + 3],
            ]);
            return Ok(iftype);
        }

        // NLA attributes are aligned to 4 bytes
        offset = (data_end + 3) & !3;
    }

    Err(CapabilityError::Provider("IFTYPE not found in GET_INTERFACE response".into()))
}

/// Query station info (connected BSS, signal, bitrate) via nl80211.
///
/// Returns (signal_dbm, tx_bitrate_kbps) if connected.
fn nl80211_get_station(ifindex: i32, bssid: &[u8; 6]) -> Result<(Option<i32>, Option<u32>), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);
    put_nla(&mut attrs, NL80211_ATTR_MAC, bssid);

    let response = sock.request(NL80211_CMD_GET_STATION, ifindex, &attrs)?;

    let msg_end = response.len();
    let genl_start = mem::size_of::<NlMsghdr>() + mem::size_of::<GenlMsghdr>();

    let mut signal_dbm: Option<i32> = None;
    let mut tx_bitrate_kbps: Option<u32> = None;

    // Parse nested station info attributes
    let mut offset = genl_start;
    while offset + mem::size_of::<NlAttr>() <= msg_end {
        let attr: NlAttr = unsafe {
            std::ptr::read_unaligned(response[offset..].as_ptr() as *const NlAttr)
        };
        let data_start = offset + mem::size_of::<NlAttr>();
        let data_end = data_start + (attr.nla_len as usize - mem::size_of::<NlAttr>());

        if data_end > msg_end {
            break;
        }

        match attr.nla_type {
            NL80211_STA_INFO_SIGNAL if data_end - data_start >= 1 => {
                // Signal is in dBm, stored as u8 (value - 256)
                let raw = response[data_start] as i8;
                signal_dbm = Some(raw as i32);
            }
            NL80211_STA_INFO_TX_BITRATE if data_end - data_start >= 4 => {
                // Nested attribute containing bitrate info
                // Parse nested sub-attributes for NL80211_RATE_INFO_BITRATE32
                let mut nested_off = data_start;
                while nested_off + mem::size_of::<NlAttr>() <= data_end {
                    let nested: NlAttr = unsafe {
                        std::ptr::read_unaligned(response[nested_off..].as_ptr() as *const NlAttr)
                    };
                    let n_start = nested_off + mem::size_of::<NlAttr>();
                    let n_end = n_start + (nested.nla_len as usize - mem::size_of::<NlAttr>());
                    if n_end > data_end {
                        break;
                    }
                    // NL80211_RATE_INFO_BITRATE32 = 7, 4-byte value in 100kbit/s
                    const NL80211_RATE_INFO_BITRATE32: u16 = 7;
                    if nested.nla_type == NL80211_RATE_INFO_BITRATE32 && n_end - n_start >= 4 {
                        let rate_100k = u32::from_ne_bytes([
                            response[n_start], response[n_start + 1],
                            response[n_start + 2], response[n_start + 3],
                        ]);
                        tx_bitrate_kbps = Some(rate_100k / 10);
                    }
                    nested_off = (n_end + 3) & !3;
                }
            }
            _ => {}
        }

        offset = (data_end + 3) & !3;
    }

    Ok((signal_dbm, tx_bitrate_kbps))
}

/// Query all connected stations (for AP mode) or get station list.
///
/// Returns Vec of (mac, signal_dbm, tx_bitrate_kbps).
fn nl80211_dump_stations(ifindex: i32) -> Result<Vec<([u8; 6], i32, Option<u32>)>, CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);

    let response = sock.request_dump(NL80211_CMD_GET_STATION, ifindex, &attrs)?;

    // Each NL message may contain multiple stations. Parse them.
    let mut stations = Vec::new();
    let mut offset = 0;
    let msg_end = response.len();

    while offset + mem::size_of::<NlMsghdr>() <= msg_end {
        let nlhdr: NlMsghdr = unsafe {
            std::ptr::read_unaligned(response[offset..].as_ptr() as *const NlMsghdr)
        };
        if nlhdr.nlmsg_len == 0 || offset + nlhdr.nlmsg_len as usize > msg_end {
            break;
        }

        let inner_end = offset + nlhdr.nlmsg_len as usize;
        let genl_start = offset + mem::size_of::<NlMsghdr>() + mem::size_of::<GenlMsghdr>();

        let mut bssid: Option<[u8; 6]> = None;
        let mut signal_dbm: Option<i32> = None;
        let mut tx_bitrate_kbps: Option<u32> = None;

        let mut attr_off = genl_start;
        while attr_off + mem::size_of::<NlAttr>() <= inner_end {
            let attr: NlAttr = unsafe {
                std::ptr::read_unaligned(response[attr_off..].as_ptr() as *const NlAttr)
            };
            let d_start = attr_off + mem::size_of::<NlAttr>();
            let d_end = d_start + (attr.nla_len as usize - mem::size_of::<NlAttr>());

            if d_end > inner_end {
                break;
            }

            match attr.nla_type {
                NL80211_ATTR_MAC if d_end - d_start >= 6 => {
                    let mut mac = [0u8; 6];
                    mac.copy_from_slice(&response[d_start..d_start + 6]);
                    bssid = Some(mac);
                }
                NL80211_STA_INFO_SIGNAL if d_end - d_start >= 1 => {
                    signal_dbm = Some(response[d_start] as i8 as i32);
                }
                NL80211_STA_INFO_TX_BITRATE if d_end - d_start >= 4 => {
                    // Parse nested rate info attributes
                    const NL80211_RATE_INFO_BITRATE32: u16 = 7;
                    let mut nested_off = d_start;
                    while nested_off + mem::size_of::<NlAttr>() <= d_end {
                        let nested: NlAttr = unsafe {
                            std::ptr::read_unaligned(response[nested_off..].as_ptr() as *const NlAttr)
                        };
                        let n_start = nested_off + mem::size_of::<NlAttr>();
                        let n_end = n_start + (nested.nla_len as usize - mem::size_of::<NlAttr>());
                        if n_end > d_end {
                            break;
                        }
                        if nested.nla_type == NL80211_RATE_INFO_BITRATE32 && n_end - n_start >= 4 {
                            let rate_100k = u32::from_ne_bytes([
                                response[n_start], response[n_start + 1],
                                response[n_start + 2], response[n_start + 3],
                            ]);
                            tx_bitrate_kbps = Some(rate_100k / 10);
                        }
                        nested_off = (n_end + 3) & !3;
                    }
                }
                _ => {}
            }

            attr_off = (d_end + 3) & !3;
        }

        if let Some(mac) = bssid {
            stations.push((mac, signal_dbm.unwrap_or(-128), tx_bitrate_kbps));
        }

        offset = (inner_end + 3) & !3;
    }

    Ok(stations)
}

/// Get the current regulatory domain (country code) via nl80211.
///
/// Returns the ISO 3166-1 alpha-2 country code (e.g. "US", "GB", "DE")
/// and the regulatory domain type.
pub fn nl80211_get_regulatory_domain() -> Result<(String, u8), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    // NL80211_CMD_GET_REG with no attributes queries the current regdomain
    let response = sock.request(NL80211_CMD_GET_REG, 0, &Vec::new())?;

    let msg_end = response.len();
    let genl_start = mem::size_of::<NlMsghdr>() + mem::size_of::<GenlMsghdr>();

    let mut country_code: Option<String> = None;
    let mut reg_type: u8 = NL80211_REGDOM_TYPE_WORLD;

    let mut offset = genl_start;
    while offset + mem::size_of::<NlAttr>() <= msg_end {
        let attr: NlAttr = unsafe {
            std::ptr::read_unaligned(response[offset..].as_ptr() as *const NlAttr)
        };
        let data_start = offset + mem::size_of::<NlAttr>();
        let data_end = data_start + (attr.nla_len as usize - mem::size_of::<NlAttr>());

        if data_end > msg_end {
            break;
        }

        match attr.nla_type {
            NL80211_ATTR_REG_ALPHA2 if data_end - data_start >= 2 => {
                // ISO 3166-1 alpha-2 country code
                if let Ok(code) = std::str::from_utf8(&response[data_start..data_start + 2]) {
                    country_code = Some(code.to_string());
                }
            }
            NL80211_ATTR_REG_INITIATOR if data_end - data_start >= 1 => {
                reg_type = response[data_start];
            }
            _ => {}
        }

        offset = (data_end + 3) & !3;
    }

    match country_code {
        Some(code) => Ok((code, reg_type)),
        None => Ok(("00".to_string(), NL80211_REGDOM_TYPE_WORLD)), // "00" = world regulatory domain
    }
}

/// Set the regulatory domain (country code) via nl80211.
///
/// The country_code must be a valid ISO 3166-1 alpha-2 code (e.g. "US", "GB", "DE").
/// This requires CAP_NET_ADMIN capability.
pub fn nl80211_set_regulatory_domain(country_code: &str) -> Result<(), CapabilityError> {
    if country_code.len() != 2 {
        return Err(CapabilityError::InvalidRequest(
            "country code must be exactly 2 characters (ISO 3166-1 alpha-2)".into(),
        ));
    }

    // Validate ASCII letters
    if !country_code.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(CapabilityError::InvalidRequest(
            "country code must contain only ASCII letters".into(),
        ));
    }

    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_string(&mut attrs, NL80211_ATTR_REG_ALPHA2, country_code.as_bytes());

    // Set by user (initiator type 1)
    put_nla_u8(&mut attrs, NL80211_ATTR_REG_INITIATOR, NL80211_REGDOM_SET_BY_USER);

    sock.request(NL80211_CMD_REQ_SET_REG, 0, &attrs)?;
    Ok(())
}

/// Query wiphy (PHY) capabilities via nl80211.
///
/// Returns (wiphy_name, supported_iftypes, max_scan_ssids) if successful.
/// The supported_iftypes is a bitmask of NL80211_IFTYPE_* values.
pub fn nl80211_get_wiphy_info(ifindex: i32) -> Result<(String, u64, Option<u32>), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);

    let response = sock.request(NL80211_CMD_GET_WIPHY, ifindex, &attrs)?;

    let msg_end = response.len();
    let genl_start = mem::size_of::<NlMsghdr>() + mem::size_of::<GenlMsghdr>();

    let mut wiphy_name: Option<String> = None;
    let mut max_scan_ssids: Option<u32> = None;
    // Supported interface types bitmask
    let supported_iftypes: u64 = (1 << NL80211_IFTYPE_STATION) | (1 << NL80211_IFTYPE_AP);

    let mut offset = genl_start;
    while offset + mem::size_of::<NlAttr>() <= msg_end {
        let attr: NlAttr = unsafe {
            std::ptr::read_unaligned(response[offset..].as_ptr() as *const NlAttr)
        };
        let data_start = offset + mem::size_of::<NlAttr>();
        let data_end = data_start + (attr.nla_len as usize - mem::size_of::<NlAttr>());

        if data_end > msg_end {
            break;
        }

        match attr.nla_type {
            NL80211_ATTR_WIPHY_NAME if data_end - data_start > 0 => {
                if let Ok(name) = std::str::from_utf8(&response[data_start..data_end]) {
                    wiphy_name = Some(name.trim_end_matches('\0').to_string());
                }
            }
            NL80211_ATTR_MAX_NUM_SCAN_SSIDS if data_end - data_start >= 4 => {
                max_scan_ssids = Some(u32::from_ne_bytes([
                    response[data_start], response[data_start + 1],
                    response[data_start + 2], response[data_start + 3],
                ]));
            }
            _ => {}
        }

        offset = (data_end + 3) & !3;
    }

    Ok((
        wiphy_name.unwrap_or_else(|| "unknown".to_string()),
        supported_iftypes,
        max_scan_ssids,
    ))
}

/// Set BSS parameters for AP mode via nl80211.
///
/// Configures beacon interval, DTIM period, and hidden SSID.
pub fn nl80211_set_bss(
    ifindex: i32,
    beacon_interval: Option<u16>,
    dtim_period: Option<u8>,
    hidden_ssid: bool,
) -> Result<(), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);

    if let Some(bi) = beacon_interval {
        put_nla_u16(&mut attrs, NL80211_ATTR_BEACON_INTERVAL, bi);
    }
    if let Some(dtim) = dtim_period {
        put_nla_u8(&mut attrs, NL80211_ATTR_DTIM_PERIOD, dtim);
    }
    if hidden_ssid {
        // Hidden SSID: set SSID to zero-length with special IE
        put_nla_string(&mut attrs, NL80211_ATTR_SSID, b"");
    }

    sock.request(NL80211_CMD_SET_BSS, ifindex, &attrs)?;
    Ok(())
}

/// Start an access point via nl80211 (NL80211_CMD_START_AP).
///
/// Sets up SSID, beacon interval, DTIM period, and cipher suites.
fn nl80211_start_ap(
    ifindex: i32,
    ssid: &[u8],
    frequency_mhz: Option<u32>,
    beacon_interval: Option<u16>,
    dtim_period: Option<u8>,
    secure: bool,
) -> Result<(), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);
    put_nla_string(&mut attrs, NL80211_ATTR_SSID, ssid);

    if let Some(freq) = frequency_mhz {
        put_nla_u32(&mut attrs, NL80211_ATTR_WIPHY_FREQ, freq);
    }

    if let Some(bi) = beacon_interval {
        put_nla_u16(&mut attrs, NL80211_ATTR_BEACON_INTERVAL, bi);
    }
    if let Some(dtim) = dtim_period {
        put_nla_u8(&mut attrs, NL80211_ATTR_DTIM_PERIOD, dtim);
    }

    // Build beacon head (minimum: management frame header + SSID IE)
    let mut beacon_head = Vec::new();
    // Management frame: subtype beacon (0x80), duration, DA (broadcast), SA, BSSID, seq
    beacon_head.push(0x80); // subtype: beacon
    beacon_head.push(0x00); // flags
    beacon_head.extend_from_slice(&[0xFFu8; 6]); // DA: broadcast
    beacon_head.extend_from_slice(&[0x00u8; 6]); // SA: placeholder (set by kernel)
    beacon_head.extend_from_slice(&[0x00u8; 6]); // BSSID: placeholder
    beacon_head.extend_from_slice(&[0x00u8; 2]); // seq ctrl

    // Timestamp (8 bytes), beacon interval (2), capability (2)
    beacon_head.extend_from_slice(&[0u8; 8]); // timestamp
    beacon_head.extend_from_slice(&beacon_interval.unwrap_or(100).to_le_bytes());
    beacon_head.extend_from_slice(&[0x11u8, 0x00]); // capability: ESS

    // SSID IE
    beacon_head.push(0); // IE type: SSID
    beacon_head.push(ssid.len() as u8);
    beacon_head.extend_from_slice(ssid);

    // RSN IE if secure
    if secure {
        let rsn_ie = build_rsn_ie();
        beacon_head.extend_from_slice(&rsn_ie);
    }

    put_nla(&mut attrs, NL80211_ATTR_BEACON_HEAD, &beacon_head);

    if secure {
        // Set cipher suites for AP mode
        let mut pairwise = Vec::new();
        pairwise.extend_from_slice(&WIFI_CIPHER_SUITE_CCMP.to_ne_bytes());
        put_nla(&mut attrs, NL80211_ATTR_CIPHER_SUITES_PAIRWISE, &pairwise);
        put_nla_u32(&mut attrs, NL80211_ATTR_CIPHER_SUITE_GROUP, WIFI_CIPHER_SUITE_CCMP);
    }

    sock.request(NL80211_CMD_START_AP, ifindex, &attrs)?;
    Ok(())
}

/// Stop the access point via nl80211 (NL80211_CMD_STOP_AP).
fn nl80211_stop_ap(ifindex: i32) -> Result<(), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);

    sock.request(NL80211_CMD_STOP_AP, ifindex, &attrs)?;
    Ok(())
}

/// Set the operating frequency via nl80211 (NL80211_CMD_SET_WIPHY).
fn nl80211_set_wiphy_freq(ifindex: i32, freq_mhz: u32) -> Result<(), CapabilityError> {
    let mut sock = Nl80211Socket::open()?;

    let mut attrs = Vec::new();
    put_nla_u32(&mut attrs, NL80211_ATTR_IFINDEX, ifindex as u32);
    put_nla_u32(&mut attrs, NL80211_ATTR_WIPHY_FREQ, freq_mhz);

    sock.request(NL80211_CMD_SET_WIPHY, ifindex, &attrs)?;
    Ok(())
}

/// Get the interface index (ifindex) for a network interface name.
pub fn get_ifindex(name: &str) -> Result<i32, CapabilityError> {
    let c_name = std::ffi::CString::new(name).map_err(|e| {
        CapabilityError::Provider(format!("invalid interface name: {}", e).into())
    })?;
    let ifindex = unsafe { libc::if_nametoindex(c_name.as_ptr()) };
    if ifindex == 0 {
        Err(CapabilityError::Provider(
            format!("interface {} not found", name).into(),
        ))
    } else {
        Ok(ifindex as i32)
    }
}

// ============================================================================
// Wireless Extensions ioctls (legacy, for basic operations)
// ============================================================================

const SIOCGIWESSID: c_ulong = 0x8B1B;
const SIOCSIWESSID: c_ulong = 0x8B1A;
const SIOCGIWAP: c_ulong = 0x8B15;
const SIOCGIWFREQ: c_ulong = 0x8B05;
const SIOCSIWFREQ: c_ulong = 0x8B04;
const SIOCGIWMODE: c_ulong = 0x8B07;
const SIOCSIWMODE: c_ulong = 0x8B06;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct IwPoint {
    pointer: *mut core::ffi::c_void,
    length: u16,
    flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct IwFreq {
    m: i32,
    e: i16,
    i: u8,
    flags: u8,
}

#[repr(C)]
pub(crate) union IwreqData {
    name: [c_char; 16],
    essid: IwPoint,
    ap_addr: [u8; 24],
    freq: IwFreq,
    mode: u32,
}

#[repr(C)]
pub(crate) struct Iwreq {
    ifr_name: [c_char; 16],
    u: IwreqData,
}

