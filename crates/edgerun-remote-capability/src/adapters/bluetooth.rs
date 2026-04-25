//! Bluetooth scanner and connection lister remote adapters.

use edgerun_bluetooth::{
    BluetoothAddressKind, BluetoothBeaconObservation, BluetoothConnectionInfo,
    BluetoothConnectionProvider, BluetoothLinkKind, BluetoothProfile, BluetoothScanResult,
    BluetoothScanner, BluetoothTransportKind,
};
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionEvent, CapabilitySessionOpen,
};

use crate::protocol::{
    accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult,
};

// --- Enum converters ---

fn bluetooth_address_kind_to_u8(kind: BluetoothAddressKind) -> u8 {
    match kind {
        BluetoothAddressKind::Public => 1,
        BluetoothAddressKind::Random => 2,
        BluetoothAddressKind::Unknown => 0,
    }
}

fn bluetooth_address_kind_from_u8(v: u8) -> Result<BluetoothAddressKind, CapabilityError> {
    Ok(match v {
        0 => BluetoothAddressKind::Unknown,
        1 => BluetoothAddressKind::Public,
        2 => BluetoothAddressKind::Random,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth address kind is invalid",
            ))
        }
    })
}

fn bluetooth_transport_kind_to_u8(kind: BluetoothTransportKind) -> u8 {
    match kind {
        BluetoothTransportKind::Classic => 1,
        BluetoothTransportKind::LowEnergy => 2,
        BluetoothTransportKind::DualMode => 3,
        BluetoothTransportKind::Unknown => 0,
    }
}

fn bluetooth_transport_kind_from_u8(v: u8) -> Result<BluetoothTransportKind, CapabilityError> {
    Ok(match v {
        0 => BluetoothTransportKind::Unknown,
        1 => BluetoothTransportKind::Classic,
        2 => BluetoothTransportKind::LowEnergy,
        3 => BluetoothTransportKind::DualMode,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth transport kind is invalid",
            ))
        }
    })
}

fn bluetooth_profile_to_u8(profile: BluetoothProfile) -> u8 {
    match profile {
        BluetoothProfile::AudioSink => 1,
        BluetoothProfile::AudioSource => 2,
        BluetoothProfile::Headset => 3,
        BluetoothProfile::HandsFree => 4,
        BluetoothProfile::HearingAid => 5,
        BluetoothProfile::Microphone => 6,
        BluetoothProfile::Speaker => 7,
        BluetoothProfile::Headphones => 8,
        BluetoothProfile::CarAudio => 9,
        BluetoothProfile::Hid => 10,
        BluetoothProfile::HeartRate => 11,
        BluetoothProfile::BatteryService => 12,
        BluetoothProfile::Other => 255,
    }
}

fn bluetooth_profile_from_u8(v: u8) -> Result<BluetoothProfile, CapabilityError> {
    Ok(match v {
        1 => BluetoothProfile::AudioSink,
        2 => BluetoothProfile::AudioSource,
        3 => BluetoothProfile::Headset,
        4 => BluetoothProfile::HandsFree,
        5 => BluetoothProfile::HearingAid,
        6 => BluetoothProfile::Microphone,
        7 => BluetoothProfile::Speaker,
        8 => BluetoothProfile::Headphones,
        9 => BluetoothProfile::CarAudio,
        10 => BluetoothProfile::Hid,
        11 => BluetoothProfile::HeartRate,
        12 => BluetoothProfile::BatteryService,
        255 => BluetoothProfile::Other,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth profile is invalid",
            ))
        }
    })
}

fn bluetooth_link_kind_to_u8(kind: BluetoothLinkKind) -> u8 {
    match kind {
        BluetoothLinkKind::Sco => 1,
        BluetoothLinkKind::Acl => 2,
        BluetoothLinkKind::Esco => 3,
        BluetoothLinkKind::Unknown => 0,
    }
}

fn bluetooth_link_kind_from_u8(v: u8) -> Result<BluetoothLinkKind, CapabilityError> {
    Ok(match v {
        0 => BluetoothLinkKind::Unknown,
        1 => BluetoothLinkKind::Sco,
        2 => BluetoothLinkKind::Acl,
        3 => BluetoothLinkKind::Esco,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth link kind is invalid",
            ))
        }
    })
}

// --- String helpers ---

fn encode_string_field(value: &str, out: &mut Vec<u8>) {
    edgerun_encoding::string_field::encode_string_field_u32(value, out)
        .expect("string field encode failed");
}

fn decode_string_field(bytes: &[u8], cursor: &mut usize) -> Result<String, CapabilityError> {
    edgerun_encoding::string_field::decode_string_field_u32(bytes, cursor)
        .map_err(|_| CapabilityError::InvalidRequest("remote string field decode failed"))
}

fn encode_optional_string_field(value: &Option<String>, out: &mut Vec<u8>) {
    edgerun_encoding::string_field::encode_optional_string_field_u32(value.as_deref(), out)
        .expect("optional string field encode failed");
}

fn decode_optional_string_field(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Option<String>, CapabilityError> {
    edgerun_encoding::string_field::decode_optional_string_field_u32(bytes, cursor)
        .map_err(|_| CapabilityError::InvalidRequest("remote optional string field decode failed"))
}

fn encode_string_vec(values: &[String], out: &mut Vec<u8>) {
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for value in values {
        encode_string_field(value, out);
    }
}

fn decode_string_vec(bytes: &[u8], cursor: &mut usize) -> Result<Vec<String>, CapabilityError> {
    if bytes.len() < *cursor + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote string vector length missing",
        ));
    }
    let count = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap()) as usize;
    *cursor += 4;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decode_string_field(bytes, cursor)?);
    }
    Ok(values)
}

fn encode_profiles_vec(values: &[BluetoothProfile], out: &mut Vec<u8>) {
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for value in values {
        out.push(bluetooth_profile_to_u8(*value));
    }
}

fn decode_profiles_vec(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Vec<BluetoothProfile>, CapabilityError> {
    if bytes.len() < *cursor + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth profile count missing",
        ));
    }
    let count = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap()) as usize;
    *cursor += 4;
    if bytes.len() < *cursor + count {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth profiles payload too short",
        ));
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(bluetooth_profile_from_u8(bytes[*cursor])?);
        *cursor += 1;
    }
    Ok(values)
}

// --- Encode/decode ---

pub fn encode_bluetooth_scan_result(scan: &BluetoothScanResult) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(scan.observations.len() as u32).to_le_bytes());
    for observation in &scan.observations {
        encode_string_field(&observation.device_id, &mut out);
        out.push(bluetooth_transport_kind_to_u8(observation.transport_kind));
        out.push(bluetooth_address_kind_to_u8(observation.address_kind));
        out.extend_from_slice(&observation.rssi_dbm.to_le_bytes());
        out.push(observation.tx_power_dbm.is_some() as u8);
        if let Some(v) = observation.tx_power_dbm {
            out.extend_from_slice(&v.to_le_bytes());
        }
        encode_optional_string_field(&observation.local_name, &mut out);
        encode_string_vec(&observation.service_uuids, &mut out);
        encode_profiles_vec(&observation.profiles, &mut out);
        out.push(observation.classic_device_class.is_some() as u8);
        if let Some(class) = observation.classic_device_class {
            out.extend_from_slice(&class.to_le_bytes());
        }
        out.extend_from_slice(&(observation.advertisement_data.len() as u32).to_le_bytes());
        out.extend_from_slice(&observation.advertisement_data);
        out.extend_from_slice(&observation.captured_at_unix_ms.to_le_bytes());
    }
    out
}

pub fn decode_bluetooth_scan_result(bytes: &[u8]) -> Result<BluetoothScanResult, CapabilityError> {
    if bytes.len() < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth scan payload too short",
        ));
    }
    let mut cursor = 0usize;
    let count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;
    let mut observations = Vec::with_capacity(count);
    for _ in 0..count {
        let device_id = decode_string_field(bytes, &mut cursor)?;
        if bytes.len() < cursor + 4 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth observation header too short",
            ));
        }
        let transport_kind = bluetooth_transport_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let address_kind = bluetooth_address_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let rssi_dbm = i16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        cursor += 2;
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth tx power flag missing",
            ));
        }
        let has_tx = bytes[cursor] != 0;
        cursor += 1;
        let tx_power_dbm = if has_tx {
            if bytes.len() < cursor + 2 {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth tx power payload too short",
                ));
            }
            let v = i16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            cursor += 2;
            Some(v)
        } else {
            None
        };
        let local_name = decode_optional_string_field(bytes, &mut cursor)?;
        let service_uuids = decode_string_vec(bytes, &mut cursor)?;
        let profiles = decode_profiles_vec(bytes, &mut cursor)?;
        if bytes.len() < cursor + 1 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth classic class flag missing",
            ));
        }
        let has_class = bytes[cursor] != 0;
        cursor += 1;
        let classic_device_class = if has_class {
            if bytes.len() < cursor + 4 {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth classic class payload too short",
                ));
            }
            let v = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            Some(v)
        } else {
            None
        };
        if bytes.len() < cursor + 4 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth advertisement length missing",
            ));
        }
        let adv_len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        if bytes.len() < cursor + adv_len + 8 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth advertisement payload too short",
            ));
        }
        let advertisement_data = bytes[cursor..cursor + adv_len].to_vec();
        cursor += adv_len;
        let captured_at_unix_ms = i64::from_le_bytes(bytes[cursor..cursor + 8].try_into().unwrap());
        cursor += 8;
        observations.push(BluetoothBeaconObservation {
            device_id,
            transport_kind,
            address_kind,
            rssi_dbm,
            tx_power_dbm,
            local_name,
            service_uuids,
            profiles,
            classic_device_class,
            advertisement_data,
            captured_at_unix_ms,
        });
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth scan payload trailing bytes are invalid",
        ));
    }
    Ok(BluetoothScanResult { observations })
}

pub fn encode_bluetooth_connections(connections: &[BluetoothConnectionInfo]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(connections.len() as u32).to_le_bytes());
    for connection in connections {
        encode_string_field(&connection.device_id, &mut out);
        out.push(bluetooth_transport_kind_to_u8(connection.transport_kind));
        out.push(bluetooth_address_kind_to_u8(connection.address_kind));
        out.push(bluetooth_link_kind_to_u8(connection.link_kind));
        out.push(connection.outbound as u8);
        out.extend_from_slice(&connection.state.to_le_bytes());
        encode_optional_string_field(&connection.local_name, &mut out);
        encode_string_vec(&connection.service_uuids, &mut out);
        encode_profiles_vec(&connection.profiles, &mut out);
        out.push(match connection.trusted {
            Some(true) => 1,
            Some(false) => 2,
            None => 0,
        });
        out.push(match connection.paired {
            Some(true) => 1,
            Some(false) => 2,
            None => 0,
        });
    }
    out
}

pub fn decode_bluetooth_connections(
    bytes: &[u8],
) -> Result<Vec<BluetoothConnectionInfo>, CapabilityError> {
    if bytes.len() < 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth connections payload too short",
        ));
    }
    let mut cursor = 0usize;
    let count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
    cursor += 4;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let device_id = decode_string_field(bytes, &mut cursor)?;
        if bytes.len() < cursor + 6 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth connection header too short",
            ));
        }
        let transport_kind = bluetooth_transport_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let address_kind = bluetooth_address_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let link_kind = bluetooth_link_kind_from_u8(bytes[cursor])?;
        cursor += 1;
        let outbound = bytes[cursor] != 0;
        cursor += 1;
        let state = u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        cursor += 2;
        let local_name = decode_optional_string_field(bytes, &mut cursor)?;
        let service_uuids = decode_string_vec(bytes, &mut cursor)?;
        let profiles = decode_profiles_vec(bytes, &mut cursor)?;
        if bytes.len() < cursor + 2 {
            return Err(CapabilityError::InvalidRequest(
                "remote bluetooth connection trust flags missing",
            ));
        }
        let trusted = match bytes[cursor] {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth trusted flag is invalid",
                ))
            }
        };
        cursor += 1;
        let paired = match bytes[cursor] {
            0 => None,
            1 => Some(true),
            2 => Some(false),
            _ => {
                return Err(CapabilityError::InvalidRequest(
                    "remote bluetooth paired flag is invalid",
                ))
            }
        };
        cursor += 1;
        out.push(BluetoothConnectionInfo {
            device_id,
            transport_kind,
            address_kind,
            link_kind,
            outbound,
            state,
            local_name,
            service_uuids,
            profiles,
            trusted,
            paired,
        });
    }
    if cursor != bytes.len() {
        return Err(CapabilityError::InvalidRequest(
            "remote bluetooth connections payload trailing bytes are invalid",
        ));
    }
    Ok(out)
}

// --- Stream error helper ---
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
            error_reason: "bluetooth remote adapter is stream-oriented; use session events".into(),
            produced_at: None,
            signature: None,
        },
        inline_payload: Vec::new(),
    }
}

// --- BluetoothRemoteAdapter ---

#[derive(Debug)]
pub struct BluetoothRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
    next_sequence_no: u64,
}

impl<D> BluetoothRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self {
            device,
            descriptor,
            next_sequence_no: 1,
        }
    }
}

impl<D> RemoteCapabilityProvider for BluetoothRemoteAdapter<D>
where
    D: BluetoothScanner,
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
            inline_payload: encode_bluetooth_scan_result(&scan),
        }))
    }
}

// --- BluetoothConnectionRemoteAdapter ---

#[derive(Debug)]
pub struct BluetoothConnectionRemoteAdapter<D> {
    pub device: D,
    descriptor: CapabilityDescriptor,
}

impl<D> BluetoothConnectionRemoteAdapter<D> {
    pub fn new(device: D, descriptor: CapabilityDescriptor) -> Self {
        Self { device, descriptor }
    }
}

impl<D> RemoteCapabilityProvider for BluetoothConnectionRemoteAdapter<D>
where
    D: BluetoothConnectionProvider,
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
        let connections = self.device.list_connections()?;
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![CapabilityEventKind::Radio as i32],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload: encode_bluetooth_connections(&connections),
        })
    }
}
