#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

#[cfg(target_os = "none")]
pub mod fs {
    pub use edgerun_linux_sysfs::fs::*;
}

#[cfg(target_os = "none")]
pub mod io {
    pub use edgerun_linux_sysfs::io::*;
}

#[cfg(target_os = "none")]
pub mod os {
    pub mod raw {
        pub use edgerun_linux_sysfs::os::raw::*;
    }
}

#[cfg(target_os = "none")]
pub mod path {
    pub use edgerun_linux_sysfs::path::{Path, PathBuf};
}

#[cfg(target_os = "none")]
pub mod option {
    pub use core::option::*;
}

#[cfg(target_os = "none")]
pub mod result {
    pub use core::result::*;
}

#[cfg(target_os = "none")]
pub mod string {
    pub use alloc::string::*;
}

#[cfg(target_os = "none")]
pub mod vec {
    pub use alloc::vec::*;
}

use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_devices::network_interface::{
    NetworkAdminState, NetworkInterfaceController, NetworkInterfaceInfo, NetworkInterfaceKind,
    NetworkLinkState, default_network_interface_descriptor,
};
use edgerun_linux_sysfs::prelude::v1::*;
use edgerun_linux_sysfs::{
    close_ioctl_fd, fill_ifr_name, ioctl_call, open_ioctl_socket, read_trimmed,
};
#[cfg(not(target_os = "none"))]
use std::fs;
#[cfg(not(target_os = "none"))]
use std::io;
use std::os::raw::{c_char, c_short, c_ulong};
use std::path::{Path, PathBuf};

const SIOCGIFFLAGS: c_ulong = 0x8913;
const SIOCSIFFLAGS: c_ulong = 0x8914;
const IFF_UP: c_short = 0x1;
const ARPHRD_ETHER: u32 = 1;
const ARPHRD_LOOPBACK: u32 = 772;
const ARPHRD_TUNNEL: u32 = 768;
const ARPHRD_TUNNEL6: u32 = 769;
const ARPHRD_IEEE80211: u32 = 801;
const ARPHRD_IEEE80211_PRISM: u32 = 802;
const ARPHRD_IEEE80211_RADIOTAP: u32 = 803;

#[repr(C)]
union Ifru {
    flags: c_short,
}

#[repr(C)]
struct Ifreq {
    name: [c_char; 16],
    ifru: Ifru,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxNetworkInterface {
    pub name: String,
    pub sysfs_path: PathBuf,
    pub kind: NetworkInterfaceKind,
    pub mac_address: Option<String>,
    pub mtu: Option<u32>,
    pub link_state: NetworkLinkState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxNetifBackend {
    pub interface: LinuxNetworkInterface,
}

fn make_ifreq(name: &str) -> Ifreq {
    let mut ifr = Ifreq {
        name: [0; 16],
        ifru: Ifru { flags: 0 },
    };
    fill_ifr_name(&mut ifr.name, name);
    ifr
}

pub fn interface_flags(name: &str) -> Result<c_short, CapabilityError> {
    let fd = open_ioctl_socket().map_err(|e| CapabilityError::Provider(e.to_string()))?;
    let mut ifr = make_ifreq(name);
    let rc = unsafe { ioctl_call(fd, SIOCGIFFLAGS, &mut ifr as *mut Ifreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if rc < 0 {
        return Err(CapabilityError::Provider(format!(
            "failed to read interface flags for {name}: {err}"
        )));
    }
    Ok(unsafe { ifr.ifru.flags })
}

pub fn set_interface_up(name: &str, enabled: bool) -> Result<(), CapabilityError> {
    let fd = open_ioctl_socket().map_err(|e| CapabilityError::Provider(e.to_string()))?;
    let mut ifr = make_ifreq(name);
    let get_rc = unsafe { ioctl_call(fd, SIOCGIFFLAGS, &mut ifr as *mut Ifreq as *mut _) };
    if get_rc < 0 {
        let err = io::Error::last_os_error();
        unsafe { close_ioctl_fd(fd) };
        return Err(CapabilityError::Provider(format!(
            "failed to read interface flags for {name}: {err}"
        )));
    }
    let mut flags = unsafe { ifr.ifru.flags };
    if enabled {
        flags |= IFF_UP;
    } else {
        flags &= !IFF_UP;
    }
    ifr.ifru = Ifru { flags };
    let set_rc = unsafe { ioctl_call(fd, SIOCSIFFLAGS, &mut ifr as *mut Ifreq as *mut _) };
    let err = io::Error::last_os_error();
    unsafe { close_ioctl_fd(fd) };
    if set_rc < 0 {
        return Err(CapabilityError::Provider(format!(
            "failed to set interface flags for {name}: {err}"
        )));
    }
    Ok(())
}

fn link_state_from_str(value: Option<String>) -> NetworkLinkState {
    match value.as_deref() {
        Some("up") => NetworkLinkState::Up,
        Some("down") => NetworkLinkState::Down,
        Some("dormant") => NetworkLinkState::Dormant,
        Some("lowerlayerdown") => NetworkLinkState::LowerLayerDown,
        Some("notpresent") => NetworkLinkState::NotPresent,
        Some("testing") => NetworkLinkState::Testing,
        _ => NetworkLinkState::Unknown,
    }
}

fn kind_from_sysfs(path: &Path) -> NetworkInterfaceKind {
    let kind = read_trimmed(&path.join("type")).and_then(|v| v.parse::<u32>().ok());
    if path.join("wireless").is_dir() {
        return NetworkInterfaceKind::Wireless;
    }
    if path.join("bridge").is_dir() {
        return NetworkInterfaceKind::Bridge;
    }
    if path.join("tun_flags").exists() {
        return NetworkInterfaceKind::Tunnel;
    }
    match kind {
        Some(ARPHRD_LOOPBACK) => NetworkInterfaceKind::Loopback,
        Some(ARPHRD_IEEE80211 | ARPHRD_IEEE80211_PRISM | ARPHRD_IEEE80211_RADIOTAP) => {
            NetworkInterfaceKind::Wireless
        }
        Some(ARPHRD_TUNNEL | ARPHRD_TUNNEL6) => NetworkInterfaceKind::Tunnel,
        Some(ARPHRD_ETHER) => {
            let name = path
                .file_name()
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with("br") {
                NetworkInterfaceKind::Bridge
            } else if name.starts_with("vlan") || name.contains('.') {
                NetworkInterfaceKind::Vlan
            } else if !path.join("device").exists() {
                NetworkInterfaceKind::Virtual
            } else {
                NetworkInterfaceKind::Ethernet
            }
        }
        _ => NetworkInterfaceKind::Unknown,
    }
}

pub fn discover_network_interfaces() -> Result<Vec<LinuxNetworkInterface>, CapabilityError> {
    discover_network_interfaces_in(Path::new("/sys/class/net"))
}

pub fn discover_network_interfaces_in(
    net_root: &Path,
) -> Result<Vec<LinuxNetworkInterface>, CapabilityError> {
    let mut out = Vec::new();
    let entries = fs::read_dir(net_root)
        .map_err(|e| CapabilityError::Provider(format!("failed to read net sysfs: {e}")))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        out.push(LinuxNetworkInterface {
            name,
            sysfs_path: path.clone(),
            kind: kind_from_sysfs(&path),
            mac_address: read_trimmed(&path.join("address")),
            mtu: read_trimmed(&path.join("mtu")).and_then(|v| v.parse::<u32>().ok()),
            link_state: link_state_from_str(read_trimmed(&path.join("operstate"))),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

impl CapabilityProvider for LinuxNetifBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_network_interface_descriptor("linux-netif", &self.interface.name)
    }
}

impl NetworkInterfaceController for LinuxNetifBackend {
    fn interface_info(&self) -> Result<NetworkInterfaceInfo, CapabilityError> {
        let flags = interface_flags(&self.interface.name)?;
        Ok(NetworkInterfaceInfo {
            provider: "linux-netif".into(),
            interface_name: self.interface.name.clone(),
            kind: self.interface.kind,
            mac_address: self.interface.mac_address.clone(),
            mtu: self.interface.mtu,
            admin_state: if flags & IFF_UP != 0 {
                NetworkAdminState::Up
            } else {
                NetworkAdminState::Down
            },
            link_state: self.interface.link_state,
        })
    }

    fn set_admin_state(
        &self,
        state: NetworkAdminState,
    ) -> Result<NetworkAdminState, CapabilityError> {
        match state {
            NetworkAdminState::Up => {
                set_interface_up(&self.interface.name, true)?;
                Ok(NetworkAdminState::Up)
            }
            NetworkAdminState::Down => {
                set_interface_up(&self.interface.name, false)?;
                Ok(NetworkAdminState::Down)
            }
            NetworkAdminState::Unknown => Err(CapabilityError::InvalidRequest(
                "cannot set network admin state to unknown",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn discover_network_interface_from_sysfs() {
        let root = temp_root("edgerun-linux-netif");
        let net = root.join("net");
        fs::create_dir_all(net.join("eth0/device")).unwrap();
        fs::write(net.join("eth0/address"), "aa:bb:cc:dd:ee:ff\n").unwrap();
        fs::write(net.join("eth0/mtu"), "1500\n").unwrap();
        fs::write(net.join("eth0/operstate"), "up\n").unwrap();
        fs::write(net.join("eth0/type"), format!("{}\n", ARPHRD_ETHER)).unwrap();
        let found = discover_network_interfaces_in(&net).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "eth0");
        assert_eq!(found[0].kind, NetworkInterfaceKind::Ethernet);
        assert_eq!(found[0].link_state, NetworkLinkState::Up);
    }

    #[test]
    fn wireless_interface_is_classified() {
        let root = temp_root("edgerun-linux-netif");
        let net = root.join("net");
        fs::create_dir_all(net.join("wlan0/wireless")).unwrap();
        fs::write(net.join("wlan0/type"), format!("{}\n", ARPHRD_IEEE80211)).unwrap();
        let found = discover_network_interfaces_in(&net).unwrap();
        assert_eq!(found[0].kind, NetworkInterfaceKind::Wireless);
    }

    // --- LinuxNetworkInterface ---

    #[test]
    fn linux_network_interface_construction() {
        let iface = LinuxNetworkInterface {
            name: "eth0".to_string(),
            sysfs_path: PathBuf::from("/sys/class/net/eth0"),
            kind: NetworkInterfaceKind::Ethernet,
            mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
            mtu: Some(1500),
            link_state: NetworkLinkState::Up,
        };
        assert_eq!(iface.name, "eth0");
        assert_eq!(iface.kind, NetworkInterfaceKind::Ethernet);
    }

    #[test]
    fn linux_network_interface_clone() {
        let iface = LinuxNetworkInterface {
            name: "v".to_string(),
            sysfs_path: PathBuf::from("/sys"),
            kind: NetworkInterfaceKind::Virtual,
            mac_address: None,
            mtu: None,
            link_state: NetworkLinkState::Unknown,
        };
        assert_eq!(iface.clone(), iface);
    }

    // --- LinuxNetifBackend ---

    #[test]
    fn linux_netif_backend_descriptor() {
        let iface = LinuxNetworkInterface {
            name: "eth0".to_string(),
            sysfs_path: PathBuf::from("/sys/class/net/eth0"),
            kind: NetworkInterfaceKind::Ethernet,
            mac_address: None,
            mtu: None,
            link_state: NetworkLinkState::Unknown,
        };
        let backend = LinuxNetifBackend { interface: iface };
        let desc = backend.descriptor();
        assert_eq!(desc.provider_name, "linux-netif");
        assert_eq!(desc.provider_instance_id, "eth0");
    }

    // --- link_state_from_str ---

    #[test]
    fn link_state_from_str_valid() {
        assert_eq!(
            link_state_from_str(Some("up".to_string())),
            NetworkLinkState::Up
        );
        assert_eq!(
            link_state_from_str(Some("down".to_string())),
            NetworkLinkState::Down
        );
        assert_eq!(
            link_state_from_str(Some("dormant".to_string())),
            NetworkLinkState::Dormant
        );
        assert_eq!(
            link_state_from_str(Some("lowerlayerdown".to_string())),
            NetworkLinkState::LowerLayerDown
        );
        assert_eq!(
            link_state_from_str(Some("notpresent".to_string())),
            NetworkLinkState::NotPresent
        );
        assert_eq!(
            link_state_from_str(Some("testing".to_string())),
            NetworkLinkState::Testing
        );
    }

    #[test]
    fn link_state_from_str_unknown() {
        assert_eq!(link_state_from_str(None), NetworkLinkState::Unknown);
        assert_eq!(
            link_state_from_str(Some("invalid".to_string())),
            NetworkLinkState::Unknown
        );
    }

    // --- kind_from_sysfs ---

    #[test]
    fn kind_loopback() {
        let root = temp_root("edgerun-linux-netif-kind");
        let path = root.join("lo");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("type"), format!("{}\n", ARPHRD_LOOPBACK)).unwrap();
        assert_eq!(kind_from_sysfs(&path), NetworkInterfaceKind::Loopback);
    }

    #[test]
    fn kind_bridge_dir() {
        let root = temp_root("edgerun-linux-netif-bridge");
        let path = root.join("br0");
        fs::create_dir_all(path.join("bridge")).unwrap();
        fs::write(path.join("type"), format!("{}\n", ARPHRD_ETHER)).unwrap();
        assert_eq!(kind_from_sysfs(&path), NetworkInterfaceKind::Bridge);
    }

    #[test]
    fn kind_tunnel() {
        let root = temp_root("edgerun-linux-netif-tunnel");
        let path = root.join("tun0");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("tun_flags"), "1\n").unwrap();
        assert_eq!(kind_from_sysfs(&path), NetworkInterfaceKind::Tunnel);
    }

    #[test]
    fn kind_vlan_by_name() {
        let root = temp_root("edgerun-linux-netif-vlan");
        let path = root.join("vlan10");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("type"), format!("{}\n", ARPHRD_ETHER)).unwrap();
        assert_eq!(kind_from_sysfs(&path), NetworkInterfaceKind::Vlan);
    }

    #[test]
    fn kind_vlan_by_dot() {
        let root = temp_root("edgerun-linux-netif-vlan-dot");
        let path = root.join("eth0.100");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("type"), format!("{}\n", ARPHRD_ETHER)).unwrap();
        assert_eq!(kind_from_sysfs(&path), NetworkInterfaceKind::Vlan);
    }

    #[test]
    fn kind_virtual_no_device() {
        let root = temp_root("edgerun-linux-netif-virtual");
        let path = root.join("veth0");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("type"), format!("{}\n", ARPHRD_ETHER)).unwrap();
        assert_eq!(kind_from_sysfs(&path), NetworkInterfaceKind::Virtual);
    }

    #[test]
    fn kind_unknown_arphrd() {
        let root = temp_root("edgerun-linux-netif-unknown");
        let path = root.join("unknown0");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("type"), "9999\n").unwrap();
        assert_eq!(kind_from_sysfs(&path), NetworkInterfaceKind::Unknown);
    }

    // --- discover_network_interfaces_in multiple sorted ---

    #[test]
    fn discover_multiple_interfaces_sorted() {
        let root = temp_root("edgerun-linux-netif-multi");
        let net = root.join("net");
        fs::create_dir_all(net.join("eth1/device")).unwrap();
        fs::write(net.join("eth1/address"), "aa:bb:cc:dd:ee:01\n").unwrap();
        fs::write(net.join("eth1/mtu"), "1500\n").unwrap();
        fs::write(net.join("eth1/operstate"), "up\n").unwrap();
        fs::write(net.join("eth1/type"), format!("{}\n", ARPHRD_ETHER)).unwrap();
        fs::create_dir_all(net.join("eth0/device")).unwrap();
        fs::write(net.join("eth0/address"), "aa:bb:cc:dd:ee:00\n").unwrap();
        fs::write(net.join("eth0/mtu"), "9000\n").unwrap();
        fs::write(net.join("eth0/operstate"), "down\n").unwrap();
        fs::write(net.join("eth0/type"), format!("{}\n", ARPHRD_ETHER)).unwrap();
        let found = discover_network_interfaces_in(&net).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "eth0");
        assert_eq!(found[1].name, "eth1");
    }

    // --- LinuxNetifBackend set_admin_state ---

    #[test]
    fn backend_set_admin_state_unknown_errors() {
        let iface = LinuxNetworkInterface {
            name: "eth0".to_string(),
            sysfs_path: PathBuf::from("/sys/class/net/eth0"),
            kind: NetworkInterfaceKind::Ethernet,
            mac_address: None,
            mtu: None,
            link_state: NetworkLinkState::Unknown,
        };
        let backend = LinuxNetifBackend { interface: iface };
        // This will fail because eth0 may not exist or need root, but it should
        // return InvalidRequest for Unknown state
        let result = backend.set_admin_state(NetworkAdminState::Unknown);
        assert!(result.is_err());
        // We can at least check that it fails
    }
}
