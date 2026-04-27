//! WiFi scanner and controller remote adapters.

use crate::prelude::v1::*;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityOperation,
};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionEvent, CapabilitySessionOpen,
};
use edgerun_wifi::{
    WifiController, WifiInterfaceInfo, WifiInterfaceMode, WifiNetworkObservation, WifiPowerState,
    WifiScanResult, WifiScanner,
};

use crate::adapters::common::{
    decode_optional_string_field, decode_string_field, encode_optional_string_field,
    encode_string_field,
};
use crate::protocol::{
    accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult,
};

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
            ))
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
            ))
        }
    })
}

// --- Encode/decode ---

pub fn encode_wifi_scan_result(scan: &WifiScanResult) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(scan.observations.len() as u32).to_le_bytes());
    for observation in &scan.observations {
        encode_string_field(&observation.interface_name, &mut out);
        encode_optional_string_field(&observation.ssid, &mut out);
        encode_optional_string_field(&observation.bssid, &mut out);
        out.push(observation.signal_dbm.is_some() as u8);
        if let Some(v) = observation.signal_dbm {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.push(observation.frequency_mhz.is_some() as u8);
        if let Some(v) = observation.frequency_mhz {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.push(match observation.secure {
            None => 0,
            Some(true) => 1,
            Some(false) => 2,
        });
        out.extend_from_slice(&observation.observed_at_unix_ms.to_le_bytes());
    }
    out
}

pub fn decode_wifi_scan_result(bytes: &[u8]) -> Result<WifiScanResult, CapabilityError> {
    if bytes.len() < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote wifi scan payload too short",
        ));
    }
    let mut cursor = 0usize;
    let count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;
    let mut observations = Vec::with_capacity(count);
    for _ in 0..count {
        let interface_name = decode_string_field(bytes, &mut cursor)?;
        let ssid = decode_optional_string_field(bytes, &mut cursor)?;
        let bssid = decode_optional_string_field(bytes, &mut cursor)?;
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi signal presence byte missing",
            ));
        }
        let signal_dbm = if bytes[cursor] != 0 {
            cursor += 1;
            if bytes.len() < cursor + 2 {
                return Err(CapabilityError::InvalidRequest(
                    "remote wifi signal payload too short",
                ));
            }
            let v = i16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            cursor += 2;
            Some(v)
        } else {
            cursor += 1;
            None
        };
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi frequency presence byte missing",
            ));
        }
        let frequency_mhz = if bytes[cursor] != 0 {
            cursor += 1;
            if bytes.len() < cursor + 4 {
                return Err(CapabilityError::InvalidRequest(
                    "remote wifi frequency payload too short",
                ));
            }
            let v = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            Some(v)
        } else {
            cursor += 1;
            None
        };
        if bytes.len() < cursor + 1 + 8 {
            return Err(CapabilityError::InvalidRequest(
                "remote wifi observation trailer too short",
            ));
        }
        let secure = match bytes[cursor] {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => {
                return Err(CapabilityError::InvalidRequest(
                    "remote wifi secure flag is invalid",
                ))
            }
        };
        cursor += 1;
        let observed_at_unix_ms = i64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
        cursor += 8;
        observations.push(WifiNetworkObservation {
            interface_name,
            ssid,
            bssid,
            signal_dbm,
            frequency_mhz,
            secure,
            observed_at_unix_ms,
        });
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote wifi scan payload trailing bytes are invalid",
        ));
    }
    Ok(WifiScanResult { observations })
}

pub fn encode_wifi_interface_info(info: &WifiInterfaceInfo) -> Vec<u8> {
    let mut out = Vec::new();
    encode_string_field(&info.provider, &mut out);
    encode_string_field(&info.interface_name, &mut out);
    encode_optional_string_field(&info.mac_address, &mut out);
    encode_optional_string_field(&info.phy_name, &mut out);
    encode_optional_string_field(&info.operstate, &mut out);
    out.push(wifi_power_state_to_u8(info.power_state));
    out.push(wifi_interface_mode_to_u8(info.mode));
    out
}

pub fn decode_wifi_interface_info(bytes: &[u8]) -> Result<WifiInterfaceInfo, CapabilityError> {
    let mut cursor = 0usize;
    let provider = decode_string_field(bytes, &mut cursor)?;
    let interface_name = decode_string_field(bytes, &mut cursor)?;
    let mac_address = decode_optional_string_field(bytes, &mut cursor)?;
    let phy_name = decode_optional_string_field(bytes, &mut cursor)?;
    let operstate = decode_optional_string_field(bytes, &mut cursor)?;
    if bytes.len() != cursor + 2 {
        return Err(CapabilityError::InvalidRequest(
            "remote wifi interface info payload length is invalid",
        ));
    }
    let power_state = wifi_power_state_from_u8(bytes[cursor])?;
    let mode = wifi_interface_mode_from_u8(bytes[cursor + 1])?;
    Ok(WifiInterfaceInfo {
        provider,
        interface_name,
        mac_address,
        phy_name,
        operstate,
        power_state,
        mode,
    })
}

// --- Stream error ---
fn stream_error(invocation: &CapabilityInvocation) -> RemoteInvocationResult {
    RemoteInvocationResult {
        result: CapabilityResult {
            result_version: 1,
            invocation_id: invocation.invocation_id.clone(),
            grant_id: invocation.grant_id.clone(),
            success: false,
            result_access_class: invocation.requested_access_class,
            produced_event_kinds: Vec::new(),
            payload_object: None,
            error_reason: "wifi remote adapter is stream-oriented; use session events".into(),
            produced_at: None,
            signature: None,
        },
        inline_payload: Vec::new(),
    }
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

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(stream_error(invocation))
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

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
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
