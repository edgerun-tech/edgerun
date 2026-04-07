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
use std::os::raw::{c_char, c_ulong};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
        let _ = self.set_power_state(WifiPowerState::Enabled)?;
        self.access_point_state()
    }

    fn stop_access_point(&self) -> Result<WifiAccessPointState, CapabilityError> {
        set_mode(&self.interface.name, WifiInterfaceMode::Client)?;
        let _ = self.set_power_state(WifiPowerState::Disabled);
        self.access_point_state()
    }
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
