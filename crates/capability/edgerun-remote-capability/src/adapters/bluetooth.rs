//! Bluetooth scanner and connection lister remote adapters.

use crate::prelude::v1::*;
use edgerun_bluetooth::{
    BluetoothAddressKind, BluetoothBeaconObservation, BluetoothConnectionInfo,
    BluetoothConnectionProvider, BluetoothLinkKind, BluetoothProfile, BluetoothScanResult,
    BluetoothScanner, BluetoothTransportKind,
};
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityInvocation, CapabilityResult,
};
use edgerun_protocols::core_protocol::protocol::capability_runtime::CapabilitySessionEvent;
use edgerun_protocols::wire::{
    RemoteBluetoothBeaconObservation as BluetoothBeaconObservationWire,
    RemoteBluetoothConnectionInfo as BluetoothConnectionInfoWire,
    RemoteBluetoothConnections as BluetoothConnectionsWire,
    RemoteBluetoothScanResult as BluetoothScanResultWire,
};

use crate::adapters::common::stream_oriented_error;
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

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
            ));
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
            ));
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
            ));
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
            ));
        }
    })
}

// --- Encode/decode ---

pub fn encode_bluetooth_scan_result(scan: &BluetoothScanResult) -> Vec<u8> {
    let wire = BluetoothScanResultWire {
        observations: scan
            .observations
            .iter()
            .map(|observation| BluetoothBeaconObservationWire {
                device_id: observation.device_id.clone(),
                transport_kind: bluetooth_transport_kind_to_u8(observation.transport_kind),
                address_kind: bluetooth_address_kind_to_u8(observation.address_kind),
                rssi_dbm: observation.rssi_dbm,
                tx_power_dbm: observation.tx_power_dbm,
                local_name: observation.local_name.clone(),
                service_uuids: observation.service_uuids.clone(),
                profiles: observation
                    .profiles
                    .iter()
                    .copied()
                    .map(bluetooth_profile_to_u8)
                    .collect(),
                classic_device_class: observation.classic_device_class,
                advertisement_data: observation.advertisement_data.clone(),
                captured_at_unix_ms: observation.captured_at_unix_ms,
            })
            .collect(),
    };
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&wire)
        .expect("bluetooth scan result must serialize through rkyv")
        .into_vec()
}

pub fn decode_bluetooth_scan_result(bytes: &[u8]) -> Result<BluetoothScanResult, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_protocols::wire::from_bytes::<
        BluetoothScanResultWire,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| CapabilityError::InvalidRequest("remote bluetooth scan payload is not rkyv"))?;
    let mut observations = Vec::with_capacity(wire.observations.len());
    for observation in wire.observations {
        observations.push(BluetoothBeaconObservation {
            device_id: observation.device_id,
            transport_kind: bluetooth_transport_kind_from_u8(observation.transport_kind)?,
            address_kind: bluetooth_address_kind_from_u8(observation.address_kind)?,
            rssi_dbm: observation.rssi_dbm,
            tx_power_dbm: observation.tx_power_dbm,
            local_name: observation.local_name,
            service_uuids: observation.service_uuids,
            profiles: observation
                .profiles
                .into_iter()
                .map(bluetooth_profile_from_u8)
                .collect::<Result<Vec<_>, _>>()?,
            classic_device_class: observation.classic_device_class,
            advertisement_data: observation.advertisement_data,
            captured_at_unix_ms: observation.captured_at_unix_ms,
        });
    }
    Ok(BluetoothScanResult { observations })
}

pub fn encode_bluetooth_connections(connections: &[BluetoothConnectionInfo]) -> Vec<u8> {
    let wire = BluetoothConnectionsWire {
        connections: connections
            .iter()
            .map(|connection| BluetoothConnectionInfoWire {
                device_id: connection.device_id.clone(),
                transport_kind: bluetooth_transport_kind_to_u8(connection.transport_kind),
                address_kind: bluetooth_address_kind_to_u8(connection.address_kind),
                link_kind: bluetooth_link_kind_to_u8(connection.link_kind),
                outbound: connection.outbound,
                state: connection.state,
                local_name: connection.local_name.clone(),
                service_uuids: connection.service_uuids.clone(),
                profiles: connection
                    .profiles
                    .iter()
                    .copied()
                    .map(bluetooth_profile_to_u8)
                    .collect(),
                trusted: connection.trusted,
                paired: connection.paired,
            })
            .collect(),
    };
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&wire)
        .expect("bluetooth connections must serialize through rkyv")
        .into_vec()
}

pub fn decode_bluetooth_connections(
    bytes: &[u8],
) -> Result<Vec<BluetoothConnectionInfo>, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_protocols::wire::from_bytes::<
        BluetoothConnectionsWire,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| {
        CapabilityError::InvalidRequest("remote bluetooth connections payload is not rkyv")
    })?;
    let mut out = Vec::with_capacity(wire.connections.len());
    for connection in wire.connections {
        out.push(BluetoothConnectionInfo {
            device_id: connection.device_id,
            transport_kind: bluetooth_transport_kind_from_u8(connection.transport_kind)?,
            address_kind: bluetooth_address_kind_from_u8(connection.address_kind)?,
            link_kind: bluetooth_link_kind_from_u8(connection.link_kind)?,
            outbound: connection.outbound,
            state: connection.state,
            local_name: connection.local_name,
            service_uuids: connection.service_uuids,
            profiles: connection
                .profiles
                .into_iter()
                .map(bluetooth_profile_from_u8)
                .collect::<Result<Vec<_>, _>>()?,
            trusted: connection.trusted,
            paired: connection.paired,
        });
    }
    Ok(out)
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

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(stream_oriented_error(invocation, "bluetooth"))
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
