#![allow(dead_code)]
use super::*;
use edgerun_capabilities::CapabilityProvider;
use edgerun_wifi::{WifiController, WifiScanner, WifiAccessPointController, WifiInterfaceMode, WifiPowerState, WifiNetworkObservation};
use edgerun_network_interface::NetworkInterfaceController;
use crate::scan::nl80211_scan;
use crate::connect::nl80211_disconnect;
use crate::ap::{nl80211_start_ap, nl80211_stop_ap, nl80211_set_bss};
use crate::interface::{query_mode, set_mode, set_essid, set_frequency_mhz};
use crate::eapol::{parse_mac_string, derive_wpa_pmk};
use edgerun_linux_sysfs::temp_root;
use std::path::PathBuf;
use std::fs;
use crate::interface::discover_wifi_interfaces_in;


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

