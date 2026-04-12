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
use crate::nla::*;

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
pub(crate) const NL80211_IFTYPE_STATION: u32 = 2;
pub(crate) const NL80211_IFTYPE_AP: u32 = 3;

// Regulatory domain types (NL80211_REGDOM_*)
const NL80211_REGDOM_SET_BY_CORE: u8 = 0;
const NL80211_REGDOM_SET_BY_USER: u8 = 1;
const NL80211_REGDOM_SET_BY_DRIVER: u8 = 2;
const NL80211_REGDOM_SET_BY_COUNTRY_IE: u8 = 3;
const NL80211_REGDOM_TYPE_COUNTRY: u8 = 0;
pub(crate) const NL80211_REGDOM_TYPE_WORLD: u8 = 1;
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
pub struct Nl80211Socket {
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
    pub(crate) fn open() -> Result<Self, CapabilityError> {
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
    pub(crate) fn request(&mut self, cmd: u8, _ifindex: i32, attrs: &[u8]) -> Result<Vec<u8>, CapabilityError> {
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
    pub(crate) fn request_dump(&mut self, cmd: u8, _ifindex: i32, attrs: &[u8]) -> Result<Vec<u8>, CapabilityError> {
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

