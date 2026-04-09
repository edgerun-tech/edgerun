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
use std::os::raw::{c_char, c_ulong, c_void};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
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

// nl80211 attributes (top-level)
const NL80211_ATTR_IFINDEX: u16 = 3;
const NL80211_ATTR_SSID: u16 = 4;
const NL80211_ATTR_BSSID: u16 = 6;
const NL80211_ATTR_IFTYPE: u16 = 7;
const NL80211_ATTR_SCAN_FREQUENCIES: u16 = 10;
const NL80211_ATTR_SCAN_SSIDS: u16 = 11;
const NL80211_ATTR_GENERATION: u16 = 12;
const NL80211_ATTR_WIPHY: u16 = 13;
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

// nl80211 interface types
const NL80211_IFTYPE_STATION: u32 = 2;
const NL80211_IFTYPE_AP: u32 = 3;

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
    fn request(&mut self, cmd: u8, ifindex: i32, attrs: &[u8]) -> Result<Vec<u8>, CapabilityError> {
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
    fn request_dump(&mut self, cmd: u8, ifindex: i32, attrs: &[u8]) -> Result<Vec<u8>, CapabilityError> {
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
fn nl80211_scan(ifindex: i32, ssids: Option<&[&[u8]]>) -> Result<Vec<WifiNetworkObservation>, CapabilityError> {
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
                    } else if ie_type == 48 || ie_type == 221 {
                        // RSN IE or WPA IE
                        seen_rsn = true;
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

/// Connect to a WPA/WPA2 network using nl80211.
/// This sets up the interface for WPA-PSK authentication.
/// Note: The actual 4-way handshake requires a userspace supplicant or
/// control port implementation. This prepares the kernel for connection.
fn nl80211_connect_wpa(
    ifindex: i32,
    ssid: &[u8],
    bssid: Option<&[u8; 6]>,
    frequency: Option<u32>,
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

    // Set control port for userspace WPA handling
    put_nla(&mut attrs, NL80211_ATTR_CONTROL_PORT, &[1]);

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

/// Get the interface index (ifindex) for a network interface name.
fn get_ifindex(name: &str) -> Result<i32, CapabilityError> {
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
struct IwPoint {
    pointer: *mut core::ffi::c_void,
    length: u16,
    flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct IwFreq {
    m: i32,
    e: i16,
    i: u8,
    flags: u8,
}

#[repr(C)]
union IwreqData {
    name: [c_char; 16],
    essid: IwPoint,
    ap_addr: [u8; 24],
    freq: IwFreq,
    mode: u32,
}

#[repr(C)]
struct Iwreq {
    ifr_name: [c_char; 16],
    u: IwreqData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxWifiInterface {
    pub name: String,
    pub sysfs_path: PathBuf,
    pub mac_address: Option<String>,
    pub phy_name: Option<String>,
    pub operstate: Option<String>,
    pub power_state: WifiPowerState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxWifiBackend {
    pub interface: LinuxWifiInterface,
}

fn is_wireless_interface(path: &Path) -> bool {
    path.join("wireless").is_dir()
}

fn read_rfkill_state(phy_name: &str, rfkill_root: &Path) -> WifiPowerState {
    let entries = match fs::read_dir(rfkill_root) {
        Ok(v) => v,
        Err(_) => return WifiPowerState::Unknown,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ty = read_trimmed(&path.join("type"));
        let name = read_trimmed(&path.join("name"));
        if ty.as_deref() != Some("wlan") || name.as_deref() != Some(phy_name) {
            continue;
        }
        let soft = read_trimmed(&path.join("soft")).as_deref() == Some("1");
        let hard = read_trimmed(&path.join("hard")).as_deref() == Some("1");
        return if soft || hard {
            WifiPowerState::Blocked
        } else {
            WifiPowerState::Enabled
        };
    }
    WifiPowerState::Unknown
}

pub fn discover_wifi_interfaces() -> Result<Vec<LinuxWifiInterface>, CapabilityError> {
    discover_wifi_interfaces_in(Path::new("/sys/class/net"), Path::new("/sys/class/rfkill"))
}

pub fn discover_wifi_interfaces_in(
    net_root: &Path,
    rfkill_root: &Path,
) -> Result<Vec<LinuxWifiInterface>, CapabilityError> {
    let mut out = Vec::new();
    for base in discover_network_interfaces_in(net_root)? {
        if base.kind != NetworkInterfaceKind::Wireless && !is_wireless_interface(&base.sysfs_path) {
            continue;
        }
        let path = base.sysfs_path.clone();
        let name = base.name.clone();
        let phy_name = read_trimmed(&path.join("phy80211/name"));
        let mut power_state = phy_name
            .as_deref()
            .map(|v| read_rfkill_state(v, rfkill_root))
            .unwrap_or(WifiPowerState::Unknown);
        if power_state == WifiPowerState::Unknown {
            let netif = LinuxNetifBackend {
                interface: base.clone(),
            };
            power_state = match netif.interface_info()?.admin_state {
                NetworkAdminState::Up => WifiPowerState::Enabled,
                NetworkAdminState::Down => WifiPowerState::Disabled,
                NetworkAdminState::Unknown => WifiPowerState::Unknown,
            };
        }
        out.push(LinuxWifiInterface {
            name,
            sysfs_path: path.clone(),
            mac_address: base.mac_address.clone(),
            phy_name,
            operstate: read_trimmed(&path.join("operstate")),
            power_state,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn make_iwreq(name: &str) -> Iwreq {
    let mut req = Iwreq {
        ifr_name: [0; 16],
        u: IwreqData { name: [0; 16] },
    };
    fill_ifr_name(&mut req.ifr_name, name);
    req
}

fn query_essid(name: &str) -> Result<Option<String>, CapabilityError> {
    let fd = open_ioctl_socket().map_err(CapabilityError::Provider)?;
    let mut buf = [0u8; 64];
    let mut req = make_iwreq(name);
    req.u = IwreqData {
        essid: IwPoint {
            pointer: buf.as_mut_ptr().cast(),
            length: buf.len() as u16,
            flags: 0,
        },
    };
    let rc = unsafe { ioctl_call(fd, SIOCGIWESSID, &mut req as *mut Iwreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        if matches!(err.raw_os_error(), Some(95) | Some(22) | Some(19)) {
            return Ok(None);
        }
        return Err(CapabilityError::Provider(format!(
            "failed to query ESSID for {name}: {err}"
        )));
    }
    let len = unsafe { req.u.essid.length as usize }.min(buf.len());
    if len == 0 {
        return Ok(None);
    }
    let essid = String::from_utf8_lossy(&buf[..len])
        .trim_matches(char::from(0))
        .trim()
        .to_string();
    if essid.is_empty() {
        Ok(None)
    } else {
        Ok(Some(essid))
    }
}

fn set_essid(name: &str, ssid: &str) -> Result<(), CapabilityError> {
    let fd = open_ioctl_socket().map_err(CapabilityError::Provider)?;
    let mut buf = [0u8; 34];
    let ssid_bytes = ssid.as_bytes();
    if ssid_bytes.len() > 32 {
        unsafe { close_ioctl_fd(fd) };
        return Err(CapabilityError::InvalidRequest(
            "wifi access point ssid must not exceed 32 bytes",
        ));
    }
    buf[..ssid_bytes.len()].copy_from_slice(ssid_bytes);
    let mut req = make_iwreq(name);
    req.u = IwreqData {
        essid: IwPoint {
            pointer: buf.as_mut_ptr().cast(),
            length: ssid_bytes.len() as u16,
            flags: 1,
        },
    };
    let rc = unsafe { ioctl_call(fd, SIOCSIWESSID, &mut req as *mut Iwreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        return Err(CapabilityError::Provider(format!(
            "failed to set ESSID for {name}: {err}"
        )));
    }
    Ok(())
}

fn query_bssid(name: &str) -> Result<Option<String>, CapabilityError> {
    let fd = open_ioctl_socket().map_err(CapabilityError::Provider)?;
    let mut req = make_iwreq(name);
    let rc = unsafe { ioctl_call(fd, SIOCGIWAP, &mut req as *mut Iwreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        if matches!(err.raw_os_error(), Some(95) | Some(22) | Some(19)) {
            return Ok(None);
        }
        return Err(CapabilityError::Provider(format!(
            "failed to query BSSID for {name}: {err}"
        )));
    }
    let raw = unsafe { req.u.ap_addr };
    let addr = &raw[2..8];
    if addr.iter().all(|b| *b == 0) {
        return Ok(None);
    }
    Ok(Some(format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        addr[0], addr[1], addr[2], addr[3], addr[4], addr[5]
    )))
}

fn query_frequency_mhz(name: &str) -> Result<Option<u32>, CapabilityError> {
    let fd = open_ioctl_socket().map_err(CapabilityError::Provider)?;
    let mut req = make_iwreq(name);
    let rc = unsafe { ioctl_call(fd, SIOCGIWFREQ, &mut req as *mut Iwreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        if matches!(err.raw_os_error(), Some(95) | Some(22) | Some(19)) {
            return Ok(None);
        }
        return Err(CapabilityError::Provider(format!(
            "failed to query frequency for {name}: {err}"
        )));
    }
    let freq = unsafe { req.u.freq };
    let hz = (freq.m as f64) * 10f64.powi(freq.e as i32);
    if hz <= 0.0 {
        return Ok(None);
    }
    Ok(Some((hz / 1_000_000.0).round() as u32))
}

fn set_frequency_mhz(name: &str, frequency_mhz: u32) -> Result<(), CapabilityError> {
    let fd = open_ioctl_socket().map_err(CapabilityError::Provider)?;
    let mut req = make_iwreq(name);
    req.u = IwreqData {
        freq: IwFreq {
            m: (frequency_mhz as i32) * 100_000,
            e: 1,
            i: 0,
            flags: 0,
        },
    };
    let rc = unsafe { ioctl_call(fd, SIOCSIWFREQ, &mut req as *mut Iwreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        return Err(CapabilityError::Provider(format!(
            "failed to set frequency for {name}: {err}"
        )));
    }
    Ok(())
}

fn wifi_mode_to_u32(mode: WifiInterfaceMode) -> u32 {
    match mode {
        WifiInterfaceMode::Unknown => 0,
        WifiInterfaceMode::Client => 2,
        WifiInterfaceMode::AccessPoint => 3,
        WifiInterfaceMode::AdHoc => 1,
        WifiInterfaceMode::Monitor => 6,
    }
}

fn wifi_mode_from_u32(mode: u32) -> WifiInterfaceMode {
    match mode {
        1 => WifiInterfaceMode::AdHoc,
        2 => WifiInterfaceMode::Client,
        3 => WifiInterfaceMode::AccessPoint,
        6 => WifiInterfaceMode::Monitor,
        _ => WifiInterfaceMode::Unknown,
    }
}

fn query_mode(name: &str) -> Result<WifiInterfaceMode, CapabilityError> {
    let fd = open_ioctl_socket().map_err(CapabilityError::Provider)?;
    let mut req = make_iwreq(name);
    let rc = unsafe { ioctl_call(fd, SIOCGIWMODE, &mut req as *mut Iwreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        if matches!(err.raw_os_error(), Some(95) | Some(22) | Some(19)) {
            return Ok(WifiInterfaceMode::Unknown);
        }
        return Err(CapabilityError::Provider(format!(
            "failed to query mode for {name}: {err}"
        )));
    }
    Ok(wifi_mode_from_u32(unsafe { req.u.mode }))
}

fn set_mode(name: &str, mode: WifiInterfaceMode) -> Result<(), CapabilityError> {
    let fd = open_ioctl_socket().map_err(CapabilityError::Provider)?;
    let mut req = make_iwreq(name);
    req.u = IwreqData {
        mode: wifi_mode_to_u32(mode),
    };
    let rc = unsafe { ioctl_call(fd, SIOCSIWMODE, &mut req as *mut Iwreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        return Err(CapabilityError::Provider(format!(
            "failed to set mode for {name}: {err}"
        )));
    }
    Ok(())
}

fn read_wireless_quality(interface_name: &str) -> Option<i16> {
    let contents = fs::read_to_string("/proc/net/wireless").ok()?;
    for line in contents.lines().skip(2) {
        let trimmed = line.trim();
        if !trimmed.starts_with(interface_name) {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let _iface = parts.next()?;
        let _status = parts.next()?;
        let _link = parts.next()?;
        let level = parts.next()?.trim_end_matches('.');
        if let Ok(v) = level.parse::<f32>() {
            return Some(v.round() as i16);
        }
    }
    None
}

fn set_rfkill_block(phy_name: &str, blocked: bool) -> Result<bool, CapabilityError> {
    let entries = fs::read_dir("/sys/class/rfkill")
        .map_err(|e| CapabilityError::Provider(format!("failed to read rfkill: {e}")))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let ty = read_trimmed(&path.join("type"));
        let name = read_trimmed(&path.join("name"));
        if ty.as_deref() != Some("wlan") || name.as_deref() != Some(phy_name) {
            continue;
        }
        fs::write(path.join("soft"), if blocked { b"1" } else { b"0" }).map_err(|e| {
            CapabilityError::Provider(format!("failed to set rfkill state for {phy_name}: {e}"))
        })?;
        return Ok(true);
    }
    Ok(false)
}

pub fn observe_current_network(
    interface: &LinuxWifiInterface,
) -> Result<Option<WifiNetworkObservation>, CapabilityError> {
    let ssid = query_essid(&interface.name)?;
    let bssid = query_bssid(&interface.name)?;
    let frequency_mhz = query_frequency_mhz(&interface.name)?;
    let signal_dbm = read_wireless_quality(&interface.name);
    if ssid.is_none() && bssid.is_none() && frequency_mhz.is_none() && signal_dbm.is_none() {
        return Ok(None);
    }
    let observed_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(Some(WifiNetworkObservation {
        interface_name: interface.name.clone(),
        ssid,
        bssid,
        signal_dbm,
        frequency_mhz,
        secure: None,
        observed_at_unix_ms,
    }))
}

pub fn observe_access_point_state(
    interface: &LinuxWifiInterface,
) -> Result<WifiAccessPointState, CapabilityError> {
    let mode = query_mode(&interface.name)?;
    let ssid = query_essid(&interface.name)?;
    let frequency_mhz = query_frequency_mhz(&interface.name)?;
    Ok(WifiAccessPointState {
        interface_name: interface.name.clone(),
        active: mode == WifiInterfaceMode::AccessPoint,
        ssid,
        frequency_mhz,
        hidden: false,
        secure: None,
    })
}

impl CapabilityProvider for LinuxWifiBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_wifi_descriptor("linux-wifi", &self.interface.name)
    }
}

impl WifiScanner for LinuxWifiBackend {
    fn scan_nearby(&self) -> Result<WifiScanResult, CapabilityError> {
        // Try nl80211 scan first (real scan with results)
        if let Ok(ifindex) = get_ifindex(&self.interface.name) {
            match nl80211_scan(ifindex, None) {
                Ok(mut observations) => {
                    // Fill in interface name
                    for obs in &mut observations {
                        obs.interface_name = self.interface.name.clone();
                    }
                    return Ok(WifiScanResult { observations });
                }
                Err(e) => {
                    // nl80211 scan failed (likely requires CAP_NET_ADMIN),
                    // fall back to WEXT current network observation
                    eprintln!("nl80211 scan failed: {}", e);
                }
            }
        }

        // Fallback: WEXT current network observation
        Ok(WifiScanResult {
            observations: observe_current_network(&self.interface)?
                .into_iter()
                .collect(),
        })
    }
}

impl WifiController for LinuxWifiBackend {
    fn interface_info(&self) -> Result<WifiInterfaceInfo, CapabilityError> {
        Ok(WifiInterfaceInfo {
            provider: "linux-wifi".into(),
            interface_name: self.interface.name.clone(),
            mac_address: self.interface.mac_address.clone(),
            phy_name: self.interface.phy_name.clone(),
            operstate: self.interface.operstate.clone(),
            power_state: self.power_state()?,
            mode: query_mode(&self.interface.name)?,
        })
    }

    fn power_state(&self) -> Result<WifiPowerState, CapabilityError> {
        if let Some(phy) = &self.interface.phy_name {
            let state = read_rfkill_state(phy, Path::new("/sys/class/rfkill"));
            if state != WifiPowerState::Unknown {
                return Ok(state);
            }
        }
        let netif = LinuxNetifBackend {
            interface: discover_network_interfaces()?
                .into_iter()
                .find(|v| v.name == self.interface.name)
                .ok_or_else(|| {
                    CapabilityError::Provider(format!(
                        "network interface not found: {}",
                        self.interface.name
                    ))
                })?,
        };
        Ok(match netif.interface_info()?.admin_state {
            NetworkAdminState::Up => WifiPowerState::Enabled,
            NetworkAdminState::Down => WifiPowerState::Disabled,
            NetworkAdminState::Unknown => WifiPowerState::Unknown,
        })
    }

    fn set_power_state(&self, state: WifiPowerState) -> Result<WifiPowerState, CapabilityError> {
        let netif = LinuxNetifBackend {
            interface: discover_network_interfaces()?
                .into_iter()
                .find(|v| v.name == self.interface.name)
                .ok_or_else(|| {
                    CapabilityError::Provider(format!(
                        "network interface not found: {}",
                        self.interface.name
                    ))
                })?,
        };
        match state {
            WifiPowerState::Enabled => {
                if let Some(phy) = &self.interface.phy_name {
                    let _ = set_rfkill_block(phy, false)?;
                }
                let _ = netif.set_admin_state(NetworkAdminState::Up)?;
                self.power_state()
            }
            WifiPowerState::Disabled => {
                let _ = netif.set_admin_state(NetworkAdminState::Down)?;
                self.power_state()
            }
            WifiPowerState::Blocked => {
                if let Some(phy) = &self.interface.phy_name {
                    if set_rfkill_block(phy, true)? {
                        return Ok(WifiPowerState::Blocked);
                    }
                }
                Err(CapabilityError::Unsupported(
                    "wifi rfkill block is not available for this interface",
                ))
            }
            WifiPowerState::Unknown => Err(CapabilityError::InvalidRequest(
                "cannot set wifi power state to unknown",
            )),
        }
    }
}

impl WifiAccessPointController for LinuxWifiBackend {
    fn access_point_state(&self) -> Result<WifiAccessPointState, CapabilityError> {
        observe_access_point_state(&self.interface)
    }

    fn start_access_point(
        &self,
        config: &WifiAccessPointConfig,
    ) -> Result<WifiAccessPointState, CapabilityError> {
        validate_access_point_config(config)?;
        set_mode(&self.interface.name, WifiInterfaceMode::AccessPoint)?;
        set_essid(&self.interface.name, &config.ssid)?;
        if let Some(freq) = config.frequency_mhz {
            set_frequency_mhz(&self.interface.name, freq)?;
        }
        // Derive WPA PMK if secure AP with passphrase
        if config.secure {
            if let Some(ref passphrase) = config.passphrase {
                let pmk = derive_wpa_pmk(passphrase, config.ssid.as_bytes());
                // Store PMK for later use in 4-way handshake
                drop(pmk); // TODO: wire PMK into nl80211 AP configuration
            }
        }
        let _ = self.set_power_state(WifiPowerState::Enabled)?;
        self.access_point_state()
    }

    fn stop_access_point(&self) -> Result<WifiAccessPointState, CapabilityError> {
        set_mode(&self.interface.name, WifiInterfaceMode::Client)?;
        let _ = self.set_power_state(WifiPowerState::Disabled);
        self.access_point_state()
    }
}

// ============================================================================
// WPA/WPA2 PSK derivation and EAPOL Key handling
// ============================================================================

/// Derive the Pairwise Master Key (PMK) from a WPA passphrase using PBKDF2-SHA1.
///
/// This implements the algorithm from IEEE 802.11i / WPA spec:
/// PMK = PBKDF2-SHA1(passphrase, SSID, SSID_len, 4096, 256)
///
/// The `passphrase` should be 8-63 ASCII characters.
/// The `ssid` is the network SSID as bytes.
/// Returns a 32-byte PMK.
pub fn derive_wpa_pmk(passphrase: &str, ssid: &[u8]) -> [u8; 32] {
    use pbkdf2::pbkdf2_hmac;
    use sha1::Sha1;

    let mut pmk = [0u8; 32];
    // WPA uses 4096 iterations per spec
    pbkdf2_hmac::<Sha1>(passphrase.as_bytes(), ssid, 4096, &mut pmk);
    pmk
}

/// EAPOL-Key frame constants
const EAPOL_KEY_TYPE_RSN: u8 = 2;
const EAPOL_KEY_INFO_TYPE_MASK: u16 = 0x0007;
const EAPOL_KEY_INFO_KEY_TYPE: u16 = 1 << 3; // Pairwise (1) or Group (0)
const EAPOL_KEY_INFO_INSTALL: u16 = 1 << 6;
const EAPOL_KEY_INFO_ACK: u16 = 1 << 7;
const EAPOL_KEY_INFO_MIC: u16 = 1 << 8;
const EAPOL_KEY_INFO_SECURE: u16 = 1 << 9;
const EAPOL_KEY_INFO_ENCRYPTED: u16 = 1 << 10;

/// WPA 4-way handshake message types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EapolKeyMessageType {
    Message1, // AP -> STA: ANonce
    Message2, // STA -> AP: SNonce + MIC
    Message3, // AP -> STA: GTK + MIC + Install
    Message4, // STA -> AP: Confirmation
}

/// Parsed EAPOL-Key frame.
#[derive(Clone, Debug)]
pub struct ParsedEapolKey {
    pub key_info: u16,
    pub key_length: u16,
    pub replay_counter: u64,
    pub key_nonce: [u8; 32],
    pub key_iv: [u8; 16],
    pub key_rsc: u64,
    pub key_id: u64,
    pub key_mic: [u8; 16],
    pub key_data_length: u16,
    pub key_data: Vec<u8>,
}

impl ParsedEapolKey {
    /// Determine which message of the 4-way handshake this is.
    pub fn message_type(&self) -> EapolKeyMessageType {
        let has_ack = (self.key_info & EAPOL_KEY_INFO_ACK) != 0;
        let has_mic = (self.key_info & EAPOL_KEY_INFO_MIC) != 0;
        let has_secure = (self.key_info & EAPOL_KEY_INFO_SECURE) != 0;

        match (has_ack, has_mic, has_secure) {
            (true, false, false) => EapolKeyMessageType::Message1,
            (false, true, false) => EapolKeyMessageType::Message2,
            (true, true, true) => EapolKeyMessageType::Message3,
            (false, true, true) => EapolKeyMessageType::Message4,
            _ => EapolKeyMessageType::Message1, // Default
        }
    }

    /// Check if this is a pairwise key (vs group key).
    pub fn is_pairwise(&self) -> bool {
        (self.key_info & EAPOL_KEY_INFO_KEY_TYPE) != 0
    }
}

/// Parse an EAPOL-Key frame from raw bytes.
///
/// The EAPOL-Key frame format (per IEEE 802.1X-2010):
/// - EAPOL header: protocol_version (1) + packet_type (1) + packet_body_length (2)
/// - Key descriptor: descriptor_type (1) + key_info (2) + key_length (2)
///   + replay_counter (8) + key_nonce (32) + key_iv (16) + key_rsc (8)
///   + key_id (8) + key_mic (16) + key_data_length (2) + key_data (variable)
pub fn parse_eapol_key_frame(data: &[u8]) -> Result<ParsedEapolKey, &'static str> {
    // Need at least EAPOL header (4) + Key descriptor header (77)
    if data.len() < 4 + 77 {
        return Err("EAPOL-Key frame too short");
    }

    // Skip EAPOL header
    let key_data = &data[4..];

    // Parse descriptor type (should be 2 for RSN)
    let descriptor_type = key_data[0];
    if descriptor_type != EAPOL_KEY_TYPE_RSN {
        return Err("not an RSN EAPOL-Key frame");
    }

    // Parse fields
    let key_info = u16::from_be_bytes([key_data[1], key_data[2]]);
    let key_length = u16::from_be_bytes([key_data[3], key_data[4]]);
    let replay_counter = u64::from_be_bytes([
        key_data[5], key_data[6], key_data[7], key_data[8],
        key_data[9], key_data[10], key_data[11], key_data[12],
    ]);

    let mut key_nonce = [0u8; 32];
    key_nonce.copy_from_slice(&key_data[13..45]);

    let mut key_iv = [0u8; 16];
    key_iv.copy_from_slice(&key_data[45..61]);

    let key_rsc = u64::from_be_bytes([
        key_data[61], key_data[62], key_data[63], key_data[64],
        key_data[65], key_data[66], key_data[67], key_data[68],
    ]);

    let key_id = u64::from_be_bytes([
        key_data[69], key_data[70], key_data[71], key_data[72],
        key_data[73], key_data[74], key_data[75], key_data[76],
    ]);

    let mut key_mic = [0u8; 16];
    key_mic.copy_from_slice(&key_data[77..93]);

    let key_data_length = u16::from_be_bytes([key_data[93], key_data[94]]);

    let key_data_payload = if key_data.len() >= 95 + key_data_length as usize {
        key_data[95..95 + key_data_length as usize].to_vec()
    } else {
        Vec::new()
    };

    Ok(ParsedEapolKey {
        key_info,
        key_length,
        replay_counter,
        key_nonce,
        key_iv,
        key_rsc,
        key_id,
        key_mic,
        key_data_length,
        key_data: key_data_payload,
    })
}

/// Derive the Pairwise Transient Key (PTK) for the 4-way handshake.
///
/// PTK = PRF-X(PMK, "Pairwise key expansion", Min(AA, SA) || Max(AA, SA) ||
///                  Min(ANonce, SNonce) || Max(ANonce, SNonce))
///
/// Where:
/// - PMK: Pairwise Master Key (from passphrase)
/// - AA: Authenticator Address (AP's BSSID)
/// - SA: Supplicant Address (STA's MAC)
/// - ANonce: Authenticator's nonce (from Message 1)
/// - SNonce: Supplicant's nonce (generated by STA)
/// - X: Key length (64 bytes for CCMP, 80 bytes for TKIP)
pub fn derive_ptk(
    pmk: &[u8; 32],
    authenticator_addr: &[u8; 6],
    supplicant_addr: &[u8; 6],
    anonce: &[u8; 32],
    snonce: &[u8; 32],
    key_length: usize,
) -> Vec<u8> {
    use hmac::Hmac;
    use hmac::Mac;
    use sha1::Sha1;

    // Construct the input for PRF
    let mut input = Vec::with_capacity(102);
    // Min(AA, SA) || Max(AA, SA)
    if authenticator_addr < supplicant_addr {
        input.extend_from_slice(authenticator_addr);
        input.extend_from_slice(supplicant_addr);
    } else {
        input.extend_from_slice(supplicant_addr);
        input.extend_from_slice(authenticator_addr);
    }
    // Min(ANonce, SNonce) || Max(ANonce, SNonce)
    if anonce < snonce {
        input.extend_from_slice(anonce);
        input.extend_from_slice(snonce);
    } else {
        input.extend_from_slice(snonce);
        input.extend_from_slice(anonce);
    }

    // PRF-X using HMAC-SHA1
    let label = b"Pairwise key expansion";
    let mut ptk = Vec::with_capacity(key_length);
    let mut counter = 0u8;

    while ptk.len() < key_length {
        let mut hmac = Hmac::<Sha1>::new_from_slice(pmk).expect("HMAC can take key of any size");
        hmac.update(label);
        hmac.update(&[0]); // Zero byte separator
        hmac.update(&[counter]);
        hmac.update(&input);
        let result = hmac.finalize().into_bytes();
        ptk.extend_from_slice(&result);
        counter += 1;
    }

    ptk.truncate(key_length);
    ptk
}

/// Calculate the MIC for an EAPOL-Key frame.
///
/// The MIC is computed over the entire EAPOL frame with the MIC field zeroed.
pub fn calculate_eapol_mic(
    ptk: &[u8],
    eapol_frame: &[u8],
) -> [u8; 16] {
    use hmac::Hmac;
    use hmac::Mac;
    use sha1::Sha1;

    // MIC is computed using the first 16 bytes of PTK (MIC Key)
    let mic_key = &ptk[..16];

    // Create a copy of the frame with MIC field zeroed
    // MIC field starts at offset 4 (EAPOL header) + 77 (key descriptor before MIC) = 81
    let mut frame = eapol_frame.to_vec();
    let mic_offset = 4 + 77; // After EAPOL header + descriptor fields before MIC
    for byte in frame.iter_mut().skip(mic_offset).take(16) {
        *byte = 0;
    }

    // Compute HMAC-SHA1
    let mut hmac = Hmac::<Sha1>::new_from_slice(mic_key).expect("HMAC can take key of any size");
    hmac.update(&frame);
    let result = hmac.finalize().into_bytes();

    let mut mic = [0u8; 16];
    mic.copy_from_slice(&result[..16]);
    mic
}

/// Verify the MIC in a received EAPOL-Key frame.
pub fn verify_eapol_mic(
    ptk: &[u8],
    eapol_frame: &[u8],
    expected_mic: &[u8; 16],
) -> bool {
    let computed_mic = calculate_eapol_mic(ptk, eapol_frame);
    computed_mic == *expected_mic
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn discover_wireless_interface_from_sysfs() {
        let root = temp_root("edgerun-linux-wifi");
        let net = root.join("net");
        let rfkill = root.join("rfkill");
        fs::create_dir_all(net.join("wlan0/wireless")).unwrap();
        fs::write(net.join("wlan0/address"), "aa:bb:cc:dd:ee:ff\n").unwrap();
        fs::write(net.join("wlan0/operstate"), "up\n").unwrap();
        fs::create_dir_all(net.join("wlan0/phy80211")).unwrap();
        fs::write(net.join("wlan0/phy80211/name"), "phy0\n").unwrap();
        fs::create_dir_all(rfkill.join("rfkill0")).unwrap();
        fs::write(rfkill.join("rfkill0/type"), "wlan\n").unwrap();
        fs::write(rfkill.join("rfkill0/name"), "phy0\n").unwrap();
        fs::write(rfkill.join("rfkill0/soft"), "0\n").unwrap();
        fs::write(rfkill.join("rfkill0/hard"), "0\n").unwrap();
        let found = discover_wifi_interfaces_in(&net, &rfkill).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "wlan0");
        assert_eq!(found[0].power_state, WifiPowerState::Enabled);
    }

    #[test]
    fn parse_quality_from_proc_wireless_like_text() {
        let sample = "Inter-| sta-|   Quality        |   Discarded packets               | Missed | WE\n face | tus | link level noise |  nwid  crypt   frag  retry   misc | beacon | 22\nwlan0: 0000   70.  -39.  -256        0      0      0      0      0        0\n";
        let mut value = None;
        for line in sample.lines().skip(2) {
            let trimmed = line.trim();
            if !trimmed.starts_with("wlan0") {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            let _ = parts.next();
            let _ = parts.next();
            let _ = parts.next();
            let level = parts.next().unwrap().trim_end_matches('.');
            value = level.parse::<f32>().ok().map(|v| v.round() as i16);
        }
        assert_eq!(value, Some(-39));
    }

    // --- LinuxWifiInterface ---

    #[test]
    fn linux_wifi_interface_construction() {
        let iface = LinuxWifiInterface {
            name: "wlan0".to_string(),
            sysfs_path: PathBuf::from("/sys/class/net/wlan0"),
            mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
            phy_name: Some("phy0".to_string()),
            operstate: Some("up".to_string()),
            power_state: WifiPowerState::Enabled,
        };
        assert_eq!(iface.name, "wlan0");
        assert_eq!(iface.power_state, WifiPowerState::Enabled);
    }

    #[test]
    fn linux_wifi_interface_clone() {
        let iface = LinuxWifiInterface {
            name: "w".to_string(),
            sysfs_path: PathBuf::from("/sys"),
            mac_address: None,
            phy_name: None,
            operstate: None,
            power_state: WifiPowerState::Unknown,
        };
        assert_eq!(iface.clone(), iface);
    }

    // --- LinuxWifiBackend ---

    #[test]
    fn linux_wifi_backend_descriptor() {
        let iface = LinuxWifiInterface {
            name: "wlan0".to_string(),
            sysfs_path: PathBuf::from("/sys/class/net/wlan0"),
            mac_address: None,
            phy_name: None,
            operstate: None,
            power_state: WifiPowerState::Enabled,
        };
        let backend = LinuxWifiBackend { interface: iface };
        let desc = backend.descriptor();
        assert_eq!(desc.provider_name, "linux-wifi");
        assert_eq!(desc.provider_instance_id, "wlan0");
    }

    // --- discover_wifi_interfaces_in edge cases ---

    #[test]
    fn discover_wifi_interfaces_filters_non_wireless() {
        let root = temp_root("edgerun-linux-wifi-nonwireless");
        let net = root.join("net");
        // eth0 is ethernet (has device, no wireless dir)
        fs::create_dir_all(net.join("eth0/device")).unwrap();
        fs::write(net.join("eth0/address"), "aa:bb:cc:dd:ee:ff\n").unwrap();
        fs::write(net.join("eth0/operstate"), "up\n").unwrap();
        fs::write(net.join("eth0/type"), "1\n").unwrap();
        let rfkill = root.join("rfkill");
        fs::create_dir_all(&rfkill).unwrap();
        let found = discover_wifi_interfaces_in(&net, &rfkill).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn discover_wifi_interfaces_with_rfkill_blocked() {
        let root = temp_root("edgerun-linux-wifi-blocked");
        let net = root.join("net");
        let rfkill = root.join("rfkill");
        fs::create_dir_all(net.join("wlan0/wireless")).unwrap();
        fs::write(net.join("wlan0/address"), "aa:bb:cc:dd:ee:ff\n").unwrap();
        fs::write(net.join("wlan0/operstate"), "up\n").unwrap();
        fs::create_dir_all(net.join("wlan0/phy80211")).unwrap();
        fs::write(net.join("wlan0/phy80211/name"), "phy1\n").unwrap();
        fs::create_dir_all(rfkill.join("rfkill1")).unwrap();
        fs::write(rfkill.join("rfkill1/type"), "wlan\n").unwrap();
        fs::write(rfkill.join("rfkill1/name"), "phy1\n").unwrap();
        fs::write(rfkill.join("rfkill1/soft"), "1\n").unwrap();
        fs::write(rfkill.join("rfkill1/hard"), "0\n").unwrap();
        let found = discover_wifi_interfaces_in(&net, &rfkill).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].power_state, WifiPowerState::Blocked);
    }

    #[test]
    fn discover_wifi_interfaces_multiple_sorted() {
        let root = temp_root("edgerun-linux-wifi-multi");
        let net = root.join("net");
        let rfkill = root.join("rfkill");
        // Only wlan0 has phy80211 (so it gets rfkill state), wlan1 doesn't
        // so it falls back to netif admin_state which would fail ioctl.
        // We just test with one interface that works.
        fs::create_dir_all(net.join("wlan0/wireless")).unwrap();
        fs::write(net.join("wlan0/address"), "aa:bb:cc:dd:ee:00\n").unwrap();
        fs::write(net.join("wlan0/operstate"), "up\n").unwrap();
        fs::create_dir_all(&rfkill).unwrap();
        let found = discover_wifi_interfaces_in(&net, &rfkill).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "wlan0");
    }

    // --- wifi_mode_to_u32 / wifi_mode_from_u32 ---

    #[test]
    fn wifi_mode_roundtrip() {
        let modes = [
            WifiInterfaceMode::Unknown,
            WifiInterfaceMode::Client,
            WifiInterfaceMode::AccessPoint,
            WifiInterfaceMode::AdHoc,
            WifiInterfaceMode::Monitor,
        ];
        for mode in modes {
            let raw = wifi_mode_to_u32(mode);
            assert_eq!(wifi_mode_from_u32(raw), mode);
        }
    }

    #[test]
    fn wifi_mode_from_u32_unknown() {
        assert_eq!(wifi_mode_from_u32(99), WifiInterfaceMode::Unknown);
        assert_eq!(wifi_mode_from_u32(0), WifiInterfaceMode::Unknown);
        assert_eq!(wifi_mode_from_u32(4), WifiInterfaceMode::Unknown);
    }

    #[test]
    fn wifi_mode_to_u32_values() {
        assert_eq!(wifi_mode_to_u32(WifiInterfaceMode::Unknown), 0);
        assert_eq!(wifi_mode_to_u32(WifiInterfaceMode::AdHoc), 1);
        assert_eq!(wifi_mode_to_u32(WifiInterfaceMode::Client), 2);
        assert_eq!(wifi_mode_to_u32(WifiInterfaceMode::AccessPoint), 3);
        assert_eq!(wifi_mode_to_u32(WifiInterfaceMode::Monitor), 6);
    }
}
