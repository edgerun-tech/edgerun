//! WiFi scanner and controller remote adapters.

use crate::prelude::v1::*;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityOperation,
};
use edgerun_core::protocol::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_core::protocol::capability_runtime::CapabilitySessionEvent;
use edgerun_wifi::{
    WifiController, WifiInterfaceInfo, WifiInterfaceMode, WifiNetworkObservation, WifiPowerState,
    WifiScanResult, WifiScanner,
};
use edgerun_wire::{
    RemoteWifiInterfaceInfo as WifiInterfaceInfoWire,
    RemoteWifiNetworkObservation as WifiNetworkObservationWire,
    RemoteWifiScanResult as WifiScanResultWire,
};

use crate::adapters::common::stream_oriented_error;
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

// --- Enum converters ---

fn wifi_power_state_to_u8(state: WifiPowerState) -> u8 {
    match state {
        WifiPowerState::Unknown => 0,
        WifiPowerState::Enabled => 1,
        WifiPowerState::Disabled => 2,
        WifiPowerState::Blocked => 3,
    }
}

fn wifi_power_state_from_u8(v: u8) -> Result<WifiPowerState, CapabilityError> {
    Ok(match v {
        0 => WifiPowerState::Unknown,
        1 => WifiPowerState::Enabled,
        2 => WifiPowerState::Disabled,
        3 => WifiPowerState::Blocked,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi power state is invalid",
            ));
        }
    })
}

fn wifi_interface_mode_to_u8(mode: WifiInterfaceMode) -> u8 {
    match mode {
        WifiInterfaceMode::Unknown => 0,
        WifiInterfaceMode::Client => 1,
        WifiInterfaceMode::AccessPoint => 2,
        WifiInterfaceMode::AdHoc => 3,
        WifiInterfaceMode::Monitor => 4,
    }
}

fn wifi_interface_mode_from_u8(v: u8) -> Result<WifiInterfaceMode, CapabilityError> {
    Ok(match v {
        0 => WifiInterfaceMode::Unknown,
        1 => WifiInterfaceMode::Client,
        2 => WifiInterfaceMode::AccessPoint,
        3 => WifiInterfaceMode::AdHoc,
        4 => WifiInterfaceMode::Monitor,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi interface mode is invalid",
            ));
        }
    })
}

// --- Encode/decode ---

pub fn encode_wifi_scan_result(scan: &WifiScanResult) -> Vec<u8> {
    let wire = WifiScanResultWire {
        observations: scan
            .observations
            .iter()
            .map(|observation| WifiNetworkObservationWire {
                interface_name: observation.interface_name.clone(),
                ssid: observation.ssid.clone(),
                bssid: observation.bssid.clone(),
                signal_dbm: observation.signal_dbm,
                frequency_mhz: observation.frequency_mhz,
                secure: observation.secure,
                observed_at_unix_ms: observation.observed_at_unix_ms,
            })
            .collect(),
    };
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&wire)
        .expect("wifi scan result must serialize through rkyv")
        .into_vec()
}

pub fn decode_wifi_scan_result(bytes: &[u8]) -> Result<WifiScanResult, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_wire::from_bytes::<WifiScanResultWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| CapabilityError::InvalidRequest("remote wifi scan payload is not rkyv"))?;
    Ok(WifiScanResult {
        observations: wire
            .observations
            .into_iter()
            .map(|observation| WifiNetworkObservation {
                interface_name: observation.interface_name,
                ssid: observation.ssid,
                bssid: observation.bssid,
                signal_dbm: observation.signal_dbm,
                frequency_mhz: observation.frequency_mhz,
                secure: observation.secure,
                observed_at_unix_ms: observation.observed_at_unix_ms,
            })
            .collect(),
    })
}

pub fn encode_wifi_interface_info(info: &WifiInterfaceInfo) -> Vec<u8> {
    let wire = WifiInterfaceInfoWire {
        provider: info.provider.clone(),
        interface_name: info.interface_name.clone(),
        mac_address: info.mac_address.clone(),
        phy_name: info.phy_name.clone(),
        operstate: info.operstate.clone(),
        power_state: wifi_power_state_to_u8(info.power_state),
        mode: wifi_interface_mode_to_u8(info.mode),
    };
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&wire)
        .expect("wifi interface info must serialize through rkyv")
        .into_vec()
}

pub fn decode_wifi_interface_info(bytes: &[u8]) -> Result<WifiInterfaceInfo, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_wire::from_bytes::<WifiInterfaceInfoWire, edgerun_wire::WireError>(&owned)
        .map_err(|_| {
        CapabilityError::InvalidRequest("remote wifi interface info payload is not rkyv")
    })?;
    Ok(WifiInterfaceInfo {
        provider: wire.provider,
        interface_name: wire.interface_name,
        mac_address: wire.mac_address,
        phy_name: wire.phy_name,
        operstate: wire.operstate,
        power_state: wifi_power_state_from_u8(wire.power_state)?,
        mode: wifi_interface_mode_from_u8(wire.mode)?,
    })
}

// --- WifiRemoteAdapter ---

#[derive(Debug)]
pub struct WifiRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> WifiRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self {
            device,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

impl<D> RemoteCapabilityProvider for WifiRemoteAdapter<D>
where
    D: WifiScanner,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(stream_oriented_error(invocation, "wifi"))
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let scan = self.device.scan_nearby()?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![CapabilityEventKind::Radio as i32],
            payload_object: None,
            inline_payload: encode_wifi_scan_result(&scan),
        }))
    }
}

// --- WifiControlRemoteAdapter ---

#[derive(Debug)]
pub struct WifiControlRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
}

impl<D> WifiControlRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self { device, descriptor }
    }
}

impl<D> RemoteCapabilityProvider for WifiControlRemoteAdapter<D>
where
    D: WifiController,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.descriptor.clone()
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let info = if invocation.operation == CapabilityOperation::Control as i32 {
            let parameters = inline_parameters.ok_or(CapabilityError::InvalidRequest(
                "wifi control invocation requires inline power state parameter",
            ))?;
            if parameters.len() != 1 {
                return Err(CapabilityError::InvalidRequest(
                    "wifi control invocation power state payload length is invalid",
                ));
            }
            let state = wifi_power_state_from_u8(parameters[0])?;
            self.device.set_power_state(state)?;
            self.device.interface_info()?
        } else {
            self.device.interface_info()?
        };
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![CapabilityEventKind::State as i32],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload: encode_wifi_interface_info(&info),
        })
    }
}
