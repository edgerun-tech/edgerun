#![allow(dead_code)]
#![allow(unused_must_use)]

use crate::interface::{LinuxWifiBackend, observe_current_network, observe_access_point_state, query_mode, read_rfkill_state, set_essid, set_frequency_mhz, set_mode, set_rfkill_block};
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_linux_sysfs::temp_root;
use edgerun_network_interface::{NetworkAdminState, NetworkInterfaceController};
use edgerun_wifi::{
    default_wifi_descriptor, validate_access_point_config, WifiAccessPointConfig,
    WifiAccessPointController, WifiAccessPointState, WifiController, WifiInterfaceInfo,
    WifiInterfaceMode, WifiNetworkObservation, WifiPowerState, WifiScanResult, WifiScanner,
};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::eapol::{parse_mac_string, derive_wpa_pmk};
use crate::scan::{nl80211_scan, build_rsn_ie, nl80211_connect_wpa};
use crate::connect::{nl80211_disconnect, nl80211_get_station, nl80211_get_interface_type, nl80211_set_interface_type, nl80211_dump_stations};
use crate::ap::{nl80211_start_ap, nl80211_stop_ap, nl80211_set_bss, nl80211_set_wiphy_freq, nl80211_get_wiphy_info, nl80211_get_regulatory_domain, nl80211_set_regulatory_domain};
use crate::getifindex::get_ifindex;
use crate::nla::*;
use crate::socket::{NL80211_IFTYPE_AP, NL80211_IFTYPE_STATION, NL80211_REGDOM_TYPE_WORLD};
use edgerun_linux_netif::{discover_network_interfaces, LinuxNetifBackend};


impl CapabilityProvider for LinuxWifiBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_wifi_descriptor("linux-wifi", &self.interface.name)
    }
}

impl WifiScanner for LinuxWifiBackend {
    fn scan_nearby(&self) -> Result<WifiScanResult, CapabilityError> {
        // Try nl80211 scan first (real scan with results)
        if let Ok(ifindex) = get_ifindex(&self.interface.name) {
            match nl80211_scan(ifindex, None, false, None) {
                Ok(mut observations) => {
                    // Fill in interface name
                    for obs in &mut observations {
                        obs.interface_name = self.interface.name.clone();
                    }
                    return Ok(WifiScanResult { observations });
                }
                Err(_e) => {
                    // nl80211 scan failed (likely requires CAP_NET_ADMIN),
                    // fall back to WEXT current network observation
                    edgerun_log::warn!("nl80211 scan failed");
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
        // Try nl80211 first for interface mode (modern path)
        let mode = if let Ok(ifindex) = get_ifindex(&self.interface.name) {
            match nl80211_get_interface_type(ifindex) {
                Ok(iftype) => match iftype {
                    NL80211_IFTYPE_STATION => WifiInterfaceMode::Client,
                    NL80211_IFTYPE_AP => WifiInterfaceMode::AccessPoint,
                    _ => query_mode(&self.interface.name).unwrap_or(WifiInterfaceMode::Client),
                },
                Err(_) => query_mode(&self.interface.name).unwrap_or(WifiInterfaceMode::Client),
            }
        } else {
            query_mode(&self.interface.name)?
        };

        Ok(WifiInterfaceInfo {
            provider: "linux-wifi".into(),
            interface_name: self.interface.name.clone(),
            mac_address: self.interface.mac_address.clone(),
            phy_name: self.interface.phy_name.clone(),
            operstate: self.interface.operstate.clone(),
            power_state: self.power_state()?,
            mode,
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

    fn connect(&self, ssid: &str, passphrase: Option<&str>) -> Result<(), CapabilityError> {
        let ifindex = get_ifindex(&self.interface.name)?;

        // Enable the interface
        let _ = self.set_power_state(WifiPowerState::Enabled)?;

        // Ensure station mode
        nl80211_set_interface_type(ifindex, NL80211_IFTYPE_STATION)?;

        if let Some(pass) = passphrase {
            // WPA-PSK connection — derive PMK and pass to kernel
            // so it handles the 4-way handshake internally
            let pmk = derive_wpa_pmk(pass, ssid.as_bytes());
            nl80211_connect_wpa(ifindex, ssid.as_bytes(), None, None, Some(&pmk))?;
        } else {
            // Open network — associate without auth
            nl80211_connect_wpa(ifindex, ssid.as_bytes(), None, None, None)?;
        }

        Ok(())
    }

    fn disconnect(&self) -> Result<(), CapabilityError> {
        let ifindex = get_ifindex(&self.interface.name)?;
        nl80211_disconnect(ifindex)
    }

    fn regulatory_domain(&self) -> Result<Option<String>, CapabilityError> {
        let (code, reg_type) = nl80211_get_regulatory_domain()?;
        // "00" means world regulatory domain — treat as None
        if code == "00" && reg_type == NL80211_REGDOM_TYPE_WORLD {
            Ok(None)
        } else {
            Ok(Some(code))
        }
    }

    fn set_regulatory_domain(&self, country_code: &str) -> Result<(), CapabilityError> {
        nl80211_set_regulatory_domain(country_code)
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
        let ifindex = get_ifindex(&self.interface.name)?;

        // Try nl80211 START_AP first (proper AP setup with beacon)
        let nl_ap_result = nl80211_start_ap(
            ifindex,
            config.ssid.as_bytes(),
            config.frequency_mhz,
            None,  // beacon interval: kernel default
            None,  // dtim period: kernel default
            config.secure,
        );

        if nl_ap_result.is_err() {
            // Fallback: switch to AP mode via nl80211, then use nl80211/WEXT for SSID/channel
            nl80211_set_interface_type(ifindex, NL80211_IFTYPE_AP)?;

            // Set SSID via WEXT (no nl80211 alternative for AP SSID without beacon)
            let _ = set_essid(&self.interface.name, &config.ssid);

            // Set frequency via nl80211 if possible, fallback to WEXT
            if let Some(freq) = config.frequency_mhz {
                if nl80211_set_wiphy_freq(ifindex, freq).is_err() {
                    let _ = set_frequency_mhz(&self.interface.name, freq);
                }
            }
        }

        // Enable the interface
        let _ = self.set_power_state(WifiPowerState::Enabled)?;

        // For WPA-protected APs, the kernel handles the 4-way handshake
        // using the RSN IE and cipher suites configured in nl80211_start_ap.
        // Per-station PMK can be set via NL80211_CMD_SET_PMK if needed
        // for dynamic key management.

        self.access_point_state()
    }

    fn stop_access_point(&self) -> Result<WifiAccessPointState, CapabilityError> {
        let ifindex = get_ifindex(&self.interface.name)?;

        // Try nl80211 STOP_AP first (proper AP teardown)
        if nl80211_stop_ap(ifindex).is_ok() {
            // Switch back to station mode
            if nl80211_set_interface_type(ifindex, NL80211_IFTYPE_STATION).is_err() {
                let _ = set_mode(&self.interface.name, WifiInterfaceMode::Client);
            }
        } else {
            // Fallback to WEXT: switch interface back to client mode
            let _ = set_mode(&self.interface.name, WifiInterfaceMode::Client);
        }

        let _ = self.set_power_state(WifiPowerState::Disabled);
        self.access_point_state()
    }
}

// ============================================================================
