extern crate alloc;
mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole, capability_descriptor,
};
use prelude::v1::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiPowerState {
    Unknown,
    Enabled,
    Disabled,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiInterfaceMode {
    Unknown,
    Client,
    AccessPoint,
    AdHoc,
    Monitor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiInterfaceInfo {
    pub provider: String,
    pub interface_name: String,
    pub mac_address: Option<String>,
    pub phy_name: Option<String>,
    pub operstate: Option<String>,
    pub power_state: WifiPowerState,
    pub mode: WifiInterfaceMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiNetworkObservation {
    pub interface_name: String,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub signal_dbm: Option<i16>,
    pub frequency_mhz: Option<u32>,
    pub secure: Option<bool>,
    pub observed_at_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiScanResult {
    pub observations: Vec<WifiNetworkObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiAccessPointConfig {
    pub ssid: String,
    pub frequency_mhz: Option<u32>,
    pub hidden: bool,
    pub secure: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiAccessPointState {
    pub interface_name: String,
    pub active: bool,
    pub ssid: Option<String>,
    pub frequency_mhz: Option<u32>,
    pub hidden: bool,
    pub secure: Option<bool>,
}

pub trait WifiController: CapabilityProvider {
    fn interface_info(&self) -> Result<WifiInterfaceInfo, edgerun_capabilities::CapabilityError>;
    fn power_state(&self) -> Result<WifiPowerState, edgerun_capabilities::CapabilityError>;
    fn set_power_state(
        &self,
        state: WifiPowerState,
    ) -> Result<WifiPowerState, edgerun_capabilities::CapabilityError>;
    fn connect(&self, ssid: &str) -> Result<(), edgerun_capabilities::CapabilityError> {
        let _ = ssid;
        Err(edgerun_capabilities::CapabilityError::Unsupported(
            "wifi connect is not supported by this backend",
        ))
    }
    fn disconnect(&self) -> Result<(), edgerun_capabilities::CapabilityError> {
        Err(edgerun_capabilities::CapabilityError::Unsupported(
            "wifi disconnect is not supported by this backend",
        ))
    }
    /// Get the current regulatory domain (country code).
    fn regulatory_domain(&self) -> Result<Option<String>, edgerun_capabilities::CapabilityError> {
        Err(edgerun_capabilities::CapabilityError::Unsupported(
            "regulatory domain query is not supported by this backend",
        ))
    }
    /// Set the regulatory domain (country code, ISO 3166-1 alpha-2).
    fn set_regulatory_domain(
        &self,
        country_code: &str,
    ) -> Result<(), edgerun_capabilities::CapabilityError> {
        let _ = country_code;
        Err(edgerun_capabilities::CapabilityError::Unsupported(
            "regulatory domain set is not supported by this backend",
        ))
    }
}

pub trait WifiScanner: CapabilityProvider {
    fn scan_nearby(&self) -> Result<WifiScanResult, edgerun_capabilities::CapabilityError>;
}

pub trait WifiAccessPointController: CapabilityProvider {
    fn access_point_state(
        &self,
    ) -> Result<WifiAccessPointState, edgerun_capabilities::CapabilityError>;
    fn start_access_point(
        &self,
        config: &WifiAccessPointConfig,
    ) -> Result<WifiAccessPointState, edgerun_capabilities::CapabilityError>;
    fn stop_access_point(
        &self,
    ) -> Result<WifiAccessPointState, edgerun_capabilities::CapabilityError>;
}

pub fn default_wifi_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio],
        &[CapabilityEventKind::Radio, CapabilityEventKind::State],
        &[
            CapabilityOperation::Query,
            CapabilityOperation::Observe,
            CapabilityOperation::Control,
            CapabilityOperation::Invoke,
        ],
        Vec::new(),
    )
}

pub fn validate_access_point_config(
    config: &WifiAccessPointConfig,
) -> Result<(), edgerun_capabilities::CapabilityError> {
    if config.ssid.trim().is_empty() {
        return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
            "wifi access point ssid must not be empty",
        ));
    }
    if config.ssid.len() > 32 {
        return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
            "wifi access point ssid must not exceed 32 bytes",
        ));
    }
    if config.secure {
        return Err(edgerun_capabilities::CapabilityError::Unsupported(
            "wifi shared-secret access points are not supported",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- WifiPowerState ---

    #[test]
    fn power_state_variants_are_distinct() {
        let variants = [
            WifiPowerState::Unknown,
            WifiPowerState::Enabled,
            WifiPowerState::Disabled,
            WifiPowerState::Blocked,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b);
                }
            }
        }
    }

    #[test]
    fn power_state_debug() {
        assert_eq!(format!("{:?}", WifiPowerState::Unknown), "Unknown");
        assert_eq!(format!("{:?}", WifiPowerState::Enabled), "Enabled");
        assert_eq!(format!("{:?}", WifiPowerState::Disabled), "Disabled");
        assert_eq!(format!("{:?}", WifiPowerState::Blocked), "Blocked");
    }

    #[test]
    fn power_state_is_copy_and_clone() {
        let state = WifiPowerState::Blocked;
        assert_eq!(state.clone(), state);
    }

    // --- WifiInterfaceMode ---

    #[test]
    fn interface_mode_variants_are_distinct() {
        let variants = [
            WifiInterfaceMode::Unknown,
            WifiInterfaceMode::Client,
            WifiInterfaceMode::AccessPoint,
            WifiInterfaceMode::AdHoc,
            WifiInterfaceMode::Monitor,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(*a, *b);
                }
            }
        }
    }

    #[test]
    fn interface_mode_debug() {
        assert_eq!(format!("{:?}", WifiInterfaceMode::Client), "Client");
        assert_eq!(
            format!("{:?}", WifiInterfaceMode::AccessPoint),
            "AccessPoint"
        );
        assert_eq!(format!("{:?}", WifiInterfaceMode::Monitor), "Monitor");
    }

    // --- WifiInterfaceInfo ---

    #[test]
    fn interface_info_construction() {
        let info = WifiInterfaceInfo {
            provider: "linux-wifi".to_string(),
            interface_name: "wlan0".to_string(),
            mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
            phy_name: Some("phy0".to_string()),
            operstate: Some("up".to_string()),
            power_state: WifiPowerState::Enabled,
            mode: WifiInterfaceMode::Client,
        };
        assert_eq!(info.interface_name, "wlan0");
        assert_eq!(info.power_state, WifiPowerState::Enabled);
        assert_eq!(info.mode, WifiInterfaceMode::Client);
    }

    #[test]
    fn interface_info_with_nones() {
        let info = WifiInterfaceInfo {
            provider: "test".to_string(),
            interface_name: "wlan1".to_string(),
            mac_address: None,
            phy_name: None,
            operstate: None,
            power_state: WifiPowerState::Unknown,
            mode: WifiInterfaceMode::Unknown,
        };
        assert!(info.mac_address.is_none());
        assert!(info.phy_name.is_none());
    }

    #[test]
    fn interface_info_clone_and_eq() {
        let info = WifiInterfaceInfo {
            provider: "p".to_string(),
            interface_name: "i".to_string(),
            mac_address: None,
            phy_name: None,
            operstate: None,
            power_state: WifiPowerState::Disabled,
            mode: WifiInterfaceMode::Unknown,
        };
        assert_eq!(info.clone(), info);
    }

    // --- WifiNetworkObservation ---

    #[test]
    fn network_observation_construction() {
        let obs = WifiNetworkObservation {
            interface_name: "wlan0".to_string(),
            ssid: Some("MyNetwork".to_string()),
            bssid: Some("aa:bb:cc:dd:ee:ff".to_string()),
            signal_dbm: Some(-45),
            frequency_mhz: Some(2437),
            secure: Some(true),
            observed_at_unix_ms: 1_000_000,
        };
        assert_eq!(obs.ssid.as_deref(), Some("MyNetwork"));
        assert_eq!(obs.frequency_mhz, Some(2437));
        assert_eq!(obs.secure, Some(true));
    }

    #[test]
    fn network_observation_all_none() {
        let obs = WifiNetworkObservation {
            interface_name: "wlan0".to_string(),
            ssid: None,
            bssid: None,
            signal_dbm: None,
            frequency_mhz: None,
            secure: None,
            observed_at_unix_ms: 0,
        };
        assert!(obs.ssid.is_none());
        assert!(obs.bssid.is_none());
    }

    #[test]
    fn network_observation_clone() {
        let obs = WifiNetworkObservation {
            interface_name: "w".to_string(),
            ssid: None,
            bssid: None,
            signal_dbm: None,
            frequency_mhz: None,
            secure: None,
            observed_at_unix_ms: 0,
        };
        assert_eq!(obs.clone(), obs);
    }

    // --- WifiScanResult ---

    #[test]
    fn scan_result_empty() {
        let result = WifiScanResult {
            observations: vec![],
        };
        assert!(result.observations.is_empty());
    }

    #[test]
    fn scan_result_with_observations() {
        let obs = WifiNetworkObservation {
            interface_name: "wlan0".to_string(),
            ssid: Some("SSID".to_string()),
            bssid: None,
            signal_dbm: Some(-60),
            frequency_mhz: Some(5180),
            secure: Some(false),
            observed_at_unix_ms: 0,
        };
        let result = WifiScanResult {
            observations: vec![obs.clone(), obs],
        };
        assert_eq!(result.observations.len(), 2);
    }

    #[test]
    fn scan_result_clone() {
        let result = WifiScanResult {
            observations: vec![],
        };
        assert_eq!(result.clone(), result);
    }

    // --- WifiAccessPointConfig ---

    #[test]
    fn ap_config_construction() {
        let config = WifiAccessPointConfig {
            ssid: "MyAP".to_string(),
            frequency_mhz: Some(2437),
            hidden: true,
            secure: false,
        };
        assert_eq!(config.ssid, "MyAP");
        assert!(config.hidden);
        assert!(!config.secure);
    }

    #[test]
    fn ap_config_defaults_for_optional() {
        let config = WifiAccessPointConfig {
            ssid: "Test".to_string(),
            frequency_mhz: None,
            hidden: false,
            secure: false,
        };
        assert!(config.frequency_mhz.is_none());
    }

    // --- WifiAccessPointState ---

    #[test]
    fn ap_state_construction() {
        let state = WifiAccessPointState {
            interface_name: "wlan0".to_string(),
            active: true,
            ssid: Some("MyAP".to_string()),
            frequency_mhz: Some(2437),
            hidden: false,
            secure: Some(false),
        };
        assert!(state.active);
        assert_eq!(state.ssid.as_deref(), Some("MyAP"));
    }

    #[test]
    fn ap_state_inactive() {
        let state = WifiAccessPointState {
            interface_name: "wlan0".to_string(),
            active: false,
            ssid: None,
            frequency_mhz: None,
            hidden: false,
            secure: None,
        };
        assert!(!state.active);
        assert!(state.ssid.is_none());
    }

    #[test]
    fn ap_state_clone() {
        let state = WifiAccessPointState {
            interface_name: "w".to_string(),
            active: false,
            ssid: None,
            frequency_mhz: None,
            hidden: false,
            secure: None,
        };
        assert_eq!(state.clone(), state);
    }

    // --- default_wifi_descriptor ---

    #[test]
    fn wifi_descriptor_has_correct_role() {
        let descriptor = default_wifi_descriptor("wifi", "wlan0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
    }

    #[test]
    fn wifi_descriptor_has_radio_and_state_modalities() {
        let descriptor = default_wifi_descriptor("wifi", "wlan0");
        assert_eq!(
            descriptor.modalities,
            vec![CapabilityModality::Radio as i32]
        );
    }

    #[test]
    fn wifi_descriptor_has_radio_and_state_events() {
        let descriptor = default_wifi_descriptor("wifi", "wlan0");
        assert!(
            descriptor
                .event_kinds
                .contains(&(CapabilityEventKind::Radio as i32))
        );
        assert!(
            descriptor
                .event_kinds
                .contains(&(CapabilityEventKind::State as i32))
        );
    }

    #[test]
    fn wifi_descriptor_has_all_operations() {
        let descriptor = default_wifi_descriptor("wifi", "wlan0");
        assert!(
            descriptor
                .operations
                .contains(&(CapabilityOperation::Query as i32))
        );
        assert!(
            descriptor
                .operations
                .contains(&(CapabilityOperation::Observe as i32))
        );
        assert!(
            descriptor
                .operations
                .contains(&(CapabilityOperation::Control as i32))
        );
        assert!(
            descriptor
                .operations
                .contains(&(CapabilityOperation::Invoke as i32))
        );
    }

    // --- validate_access_point_config ---

    #[test]
    fn valid_config_passes_validation() {
        let config = WifiAccessPointConfig {
            ssid: "ValidSSID".to_string(),
            frequency_mhz: None,
            hidden: false,
            secure: false,
        };
        assert!(validate_access_point_config(&config).is_ok());
    }

    #[test]
    fn empty_ssid_fails_validation() {
        let err = validate_access_point_config(&WifiAccessPointConfig {
            ssid: String::new(),
            frequency_mhz: None,
            hidden: false,
            secure: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            edgerun_capabilities::CapabilityError::InvalidRequest(
                "wifi access point ssid must not be empty"
            )
        );
    }

    #[test]
    fn whitespace_only_ssid_fails_validation() {
        let err = validate_access_point_config(&WifiAccessPointConfig {
            ssid: "   ".to_string(),
            frequency_mhz: None,
            hidden: false,
            secure: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            edgerun_capabilities::CapabilityError::InvalidRequest(
                "wifi access point ssid must not be empty"
            )
        );
    }

    #[test]
    fn ssid_too_long_fails_validation() {
        let err = validate_access_point_config(&WifiAccessPointConfig {
            ssid: "a".repeat(33),
            frequency_mhz: None,
            hidden: false,
            secure: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            edgerun_capabilities::CapabilityError::InvalidRequest(
                "wifi access point ssid must not exceed 32 bytes"
            )
        );
    }

    #[test]
    fn ssid_exactly_32_chars_is_valid() {
        let config = WifiAccessPointConfig {
            ssid: "a".repeat(32),
            frequency_mhz: None,
            hidden: false,
            secure: false,
        };
        assert!(validate_access_point_config(&config).is_ok());
    }

    #[test]
    fn secure_config_fails() {
        let err = validate_access_point_config(&WifiAccessPointConfig {
            ssid: "MyAP".to_string(),
            frequency_mhz: None,
            hidden: false,
            secure: true,
        })
        .unwrap_err();
        assert_eq!(
            err,
            edgerun_capabilities::CapabilityError::Unsupported(
                "wifi shared-secret access points are not supported"
            )
        );
    }
}
