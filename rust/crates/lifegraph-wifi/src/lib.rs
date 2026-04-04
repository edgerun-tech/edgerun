use lifegraph_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

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
    fn interface_info(&self) -> Result<WifiInterfaceInfo, lifegraph_capabilities::CapabilityError>;
    fn power_state(&self) -> Result<WifiPowerState, lifegraph_capabilities::CapabilityError>;
    fn set_power_state(
        &self,
        state: WifiPowerState,
    ) -> Result<WifiPowerState, lifegraph_capabilities::CapabilityError>;
}

pub trait WifiScanner: CapabilityProvider {
    fn scan_nearby(&self) -> Result<WifiScanResult, lifegraph_capabilities::CapabilityError>;
}

pub trait WifiAccessPointController: CapabilityProvider {
    fn access_point_state(
        &self,
    ) -> Result<WifiAccessPointState, lifegraph_capabilities::CapabilityError>;
    fn start_access_point(
        &self,
        config: &WifiAccessPointConfig,
    ) -> Result<WifiAccessPointState, lifegraph_capabilities::CapabilityError>;
    fn stop_access_point(
        &self,
    ) -> Result<WifiAccessPointState, lifegraph_capabilities::CapabilityError>;
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
) -> Result<(), lifegraph_capabilities::CapabilityError> {
    if config.ssid.trim().is_empty() {
        return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
            "wifi access point ssid must not be empty",
        ));
    }
    if config.ssid.len() > 32 {
        return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
            "wifi access point ssid must not exceed 32 bytes",
        ));
    }
    if config.secure {
        return Err(lifegraph_capabilities::CapabilityError::Unsupported(
            "secure wifi access point setup is not implemented yet",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wifi_descriptor_kind_is_correct() {
        let descriptor = default_wifi_descriptor("wifi", "wlan0");
        assert_eq!(descriptor.role, CapabilityRole::Communication as i32);
        assert!(descriptor
            .event_kinds
            .contains(&(CapabilityEventKind::Radio as i32)));
    }

    #[test]
    fn access_point_config_requires_valid_ssid() {
        let err = validate_access_point_config(&WifiAccessPointConfig {
            ssid: String::new(),
            frequency_mhz: None,
            hidden: false,
            secure: false,
        })
        .unwrap_err();
        assert_eq!(
            err,
            lifegraph_capabilities::CapabilityError::InvalidRequest(
                "wifi access point ssid must not be empty"
            )
        );
    }
}
