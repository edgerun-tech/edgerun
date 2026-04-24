use crate::error::{GattError, GattResult};
use crate::{
    format_gatt_uuid, parse_gatt_uuid, AttProtocol, GattAddressKind, GattCharacteristic,
    GattClient, GattConnectionState, GattDescriptor, GattEventCallback,
    GattEventKind, GattProperty, GattService, GattUuid, L2capSocket, UUID_CLIENT_CHARACTERISTIC_CONFIGURATION,
};
use edgerun_capabilities::{CapabilityError, CapabilityProvider};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

pub struct LinuxGattClient {
    device_addr: RwLock<Option<String>>,
    addr_type: RwLock<u8>,
    connection_state: RwLock<GattConnectionState>,
    socket: RwLock<Option<L2capSocket>>,
    mtu: RwLock<u16>,
    services: RwLock<Vec<GattService>>,
    characteristics: RwLock<HashMap<u16, GattCharacteristic>>,
    descriptors: RwLock<HashMap<u16, Vec<GattDescriptor>>>,
    notification_handlers: RwLock<HashMap<u16, Vec<GattEventCallback>>>,
    connection_callbacks: RwLock<Vec<GattEventCallback>>,
}

impl LinuxGattClient {
    pub fn new() -> Self {
        Self {
            device_addr: RwLock::new(None),
            addr_type: RwLock::new(0),
            connection_state: RwLock::new(GattConnectionState::Disconnected),
            socket: RwLock::new(None),
            mtu: RwLock::new(512),
            services: RwLock::new(Vec::new()),
            characteristics: RwLock::new(HashMap::new()),
            descriptors: RwLock::new(HashMap::new()),
            notification_handlers: RwLock::new(HashMap::new()),
            connection_callbacks: RwLock::new(Vec::new()),
        }
    }

    pub fn with_mtu(mtu: u16) -> Self {
        let client = Self::new();
        *client.mtu.write().unwrap() = mtu;
        client
    }

    fn with_protocol<F, T>(&self, f: F) -> Result<T, CapabilityError>
    where
        F: FnOnce(&mut AttProtocol) -> Result<T, GattError>,
    {
        let socket_guard = self.socket.read().unwrap();
        let socket = socket_guard
            .as_ref()
            .ok_or_else(|| -> edgerun_capabilities::CapabilityError { GattError::NotConnected.into() })?;
        let mut proto = AttProtocol::new(socket.clone());
        let mtu = *self.mtu.read().unwrap();
        proto.set_mtu(mtu);
        f(&mut proto).map_err(Into::into)
    }

    fn set_connection_state(&self, state: GattConnectionState) {
        let is_disconnecting = matches!(state, GattConnectionState::Disconnected);
        *self.connection_state.write().unwrap() = state;
        if is_disconnecting {
            self.emit_event(GattEventKind::Disconnected);
        }
    }

    fn emit_event(&self, event: GattEventKind) {
        let callbacks = self.connection_callbacks.read().unwrap();
        for callback in callbacks.iter() {
            callback(event.clone());
        }
    }

    fn emit_notification(&self, handle: u16, value: Vec<u8>) {
        let handlers = self.notification_handlers.read().unwrap();
        if let Some(callbacks) = handlers.get(&handle) {
            for callback in callbacks {
                callback(GattEventKind::Notification { handle, value: value.clone() });
            }
        }
    }

    fn parse_characteristic_properties(flags: u8) -> Vec<GattProperty> {
        GattProperty::from_bits(flags)
    }

    fn uuid_bytes(uuid: &GattUuid) -> Result<Vec<u8>, GattError> {
        let cleaned = uuid.0.replace('-', "").to_lowercase();
        match cleaned.len() {
            4 => {
                let val = u16::from_str_radix(&cleaned, 16)
                    .map_err(|_| GattError::InvalidUuid(uuid.0.clone()))?;
                Ok(val.to_le_bytes().to_vec())
            }
            32 => {
                let mut bytes = Vec::with_capacity(16);
                for i in (0..32).step_by(2) {
                    let byte = u8::from_str_radix(&cleaned[i..i + 2], 16)
                        .map_err(|_| GattError::InvalidUuid(uuid.0.clone()))?;
                    bytes.push(byte);
                }
                Ok(bytes)
            }
            _ => Err(GattError::InvalidUuid(uuid.0.clone())),
        }
    }

    pub fn on_event<F>(&self, callback: F)
    where
        F: Fn(GattEventKind) + Send + Sync + 'static
    {
        self.connection_callbacks
            .write()
            .unwrap()
            .push(Box::new(callback));
    }

    pub fn on_notification(&self, handle: u16, callback: impl Fn(GattEventKind) + Send + Sync + 'static) {
        self.notification_handlers
            .write()
            .unwrap()
            .entry(handle)
            .or_default()
            .push(Box::new(callback));
    }

    pub fn remove_notification_handler(&self, handle: u16) {
        self.notification_handlers.write().unwrap().remove(&handle);
    }

    pub fn clear_all_handlers(&self) {
        self.notification_handlers.write().unwrap().clear();
        self.connection_callbacks.write().unwrap().clear();
    }
}

impl Default for LinuxGattClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityProvider for LinuxGattClient {
    fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
        let device = self.device_addr.read().unwrap();
        let instance = device.clone().unwrap_or_else(|| "disconnected".to_string());
        crate::default_gatt_descriptor("linux-gatt", &instance)
    }
}

impl GattClient for LinuxGattClient {
    fn connect(&self, device_addr: &str, addr_type: GattAddressKind) -> Result<(), CapabilityError> {
        if device_addr.is_empty() {
            return Err(GattError::InvalidParameter("device address required".to_string()).into());
        }

        let addr_type_val = match addr_type {
            GattAddressKind::Public => 0x01,
            GattAddressKind::Random => 0x02,
            _ => 0x00,
        };

        *self.connection_state.write().unwrap() = GattConnectionState::Connecting;

        let socket = L2capSocket::new()?;

        socket.connect_device(device_addr, addr_type_val)?;

        *self.device_addr.write().unwrap() = Some(device_addr.to_string());
        *self.addr_type.write().unwrap() = addr_type_val;
        *self.socket.write().unwrap() = Some(socket);
        *self.connection_state.write().unwrap() = GattConnectionState::Connected;

        self.emit_event(GattEventKind::Connected);

        Ok(())
    }

    fn disconnect(&self) -> Result<(), CapabilityError> {
        *self.connection_state.write().unwrap() = GattConnectionState::Disconnecting;
        *self.socket.write().unwrap() = None;
        *self.device_addr.write().unwrap() = None;
        self.services.write().unwrap().clear();
        self.characteristics.write().unwrap().clear();
        self.descriptors.write().unwrap().clear();
        *self.connection_state.write().unwrap() = GattConnectionState::Disconnected;
        self.emit_event(GattEventKind::Disconnected);
        Ok(())
    }

    fn discover_services(&self) -> Result<Vec<GattService>, CapabilityError> {
        self.with_protocol(|proto| {
            let primary_group_type: [u8; 2] = [0x00, 0x28];
            let data = proto.read_by_group_type(0x0001, 0xFFFF, &primary_group_type)?;

            let mut services = Vec::new();
            for (start, end, uuid) in proto.parse_read_by_group_response(&data) {
                services.push(GattService {
                    uuid: GattUuid(format_gatt_uuid(&uuid)),
                    primary: true,
                    handle: start,
                    end_handle: end,
                });
            }

            *self.services.write().unwrap() = services.clone();
            self.emit_event(GattEventKind::ServiceDiscovered(services.clone()));

            Ok(services)
        })
    }

    fn discover_characteristics_by_range(
        &self,
        start: u16,
        end: u16,
    ) -> Result<Vec<GattCharacteristic>, CapabilityError> {
        self.with_protocol(|proto| {
            let char_type: [u8; 2] = [0x03, 0x28];
            let data = proto.read_by_type(start, end, &char_type)?;

            let mut characteristics = Vec::new();
            for (handle, value) in proto.parse_read_by_type_response(&data) {
                if value.len() < 3 {
                    continue;
                }
                let props = Self::parse_characteristic_properties(value[0]);
                let uuid = value[2..].to_vec();
                let char_uuid = GattUuid(format_gatt_uuid(&uuid));
                characteristics.push(GattCharacteristic {
                    uuid: char_uuid.clone(),
                    properties: props,
                    value_handle: handle,
                    handle,
                    permissions: Vec::new(),
                });

                self.characteristics
                    .write()
                    .unwrap()
                    .insert(handle, characteristics.last().unwrap().clone());
            }

            self.emit_event(GattEventKind::CharacteristicDiscovered(characteristics.clone()));

            Ok(characteristics)
        })
    }

    fn discover_descriptors(&self, char_handle: u16) -> Result<Vec<GattDescriptor>, CapabilityError> {
        self.with_protocol(|proto| {
            let data = proto.find_information(char_handle + 1, 0xFFFF)?;

            let mut descriptors = Vec::new();
            for (handle, uuid) in proto.parse_find_information_response(&data) {
                let desc = GattDescriptor {
                    uuid: GattUuid(format_gatt_uuid(&uuid)),
                    handle,
                    permissions: Vec::new(),
                };
                descriptors.push(desc);
            }

            self.descriptors
                .write()
                .unwrap()
                .insert(char_handle, descriptors.clone());
            self.emit_event(GattEventKind::DescriptorDiscovered(descriptors.clone()));

            Ok(descriptors)
        })
    }

    fn read_value(&self, handle: u16) -> Result<Vec<u8>, CapabilityError> {
        self.with_protocol(|proto| {
            let result = proto.read_value(handle)?;
            self.emit_event(GattEventKind::ReadResponse { handle, value: result.clone() });
            Ok(result)
        })
    }

    fn write_value(
        &self,
        handle: u16,
        data: &[u8],
        with_response: bool,
    ) -> Result<(), CapabilityError> {
        self.with_protocol(|proto| {
            proto.write_value(handle, data, with_response)?;
            if with_response {
                self.emit_event(GattEventKind::WriteResponse { handle });
            }
            Ok(())
        })
    }

    fn enable_notifications(&self, handle: u16, enable: bool) -> Result<(), CapabilityError> {
        let desc_handle = self.find_descriptor_by_uuid(handle, UUID_CLIENT_CHARACTERISTIC_CONFIGURATION)?;
        let config: [u8; 2] = if enable { [0x01, 0x00] } else { [0x00, 0x00] };
        self.write_value(desc_handle, &config, true)
    }

    fn read_by_type(&self, start: u16, end: u16, uuid: &GattUuid) -> Result<Vec<u8>, CapabilityError> {
        let uuid_bytes = Self::uuid_bytes(uuid)?;
        self.with_protocol(|proto| {
            proto.read_by_type(start, end, &uuid_bytes)})
    }

    fn write_cmd(&self, handle: u16, data: &[u8]) -> Result<(), CapabilityError> {
        self.with_protocol(|proto| {
            proto.write_cmd(handle, data)})
    }
}

impl LinuxGattClient {
    pub fn find_descriptor_by_uuid(&self, char_handle: u16, target_uuid: u16) -> Result<u16, CapabilityError> {
        let descriptors = self.discover_descriptors(char_handle)?;
        let target_bytes = target_uuid.to_le_bytes();
        let target_str = format!("{:02x}{:02x}", target_bytes[0], target_bytes[1]);

        for desc in descriptors {
            let cleaned = desc.uuid.0.replace('-', "");
            if cleaned.ends_with(&target_str) {
                return Ok(desc.handle);
            }
        }
        Err(GattError::DescriptorNotFound(format!("0x{:04x}", target_uuid)).into())
    }

    pub fn discover_all_characteristics(&self) -> Result<Vec<GattCharacteristic>, CapabilityError> {
        let services = self.discover_services()?;
        let mut all_chars = Vec::new();

        for svc in services {
            let chars = self.discover_characteristics_by_range(svc.handle, svc.end_handle)?;
            all_chars.extend(chars);
        }

        Ok(all_chars)
    }

    pub fn discover_all_descriptors(&self) -> Result<(), CapabilityError> {
        let chars = self.discover_all_characteristics()?;
        for ch in chars {
            self.discover_descriptors(ch.handle)?;
        }
        Ok(())
    }

    pub fn read_characteristic_by_uuid(&self, uuid: &str) -> Result<Vec<u8>, CapabilityError> {
        let uuid = parse_gatt_uuid(uuid)
            .ok_or_else(|| GattError::InvalidUuid(uuid.to_string()))?;

        for svc in self.services.read().unwrap().iter() {
            let chars = self.discover_characteristics_by_range(svc.handle, svc.end_handle)?;
            for ch in chars {
                if ch.uuid.0 == uuid.0 {
                    return self.read_value(ch.value_handle);
                }
            }
        }

        Err(GattError::CharacteristicNotFound(uuid.0).into())
    }

    pub fn write_characteristic_by_uuid(&self, uuid: &str, data: &[u8], with_response: bool) -> Result<(), CapabilityError> {
        let uuid = parse_gatt_uuid(uuid)
            .ok_or_else(|| GattError::InvalidUuid(uuid.to_string()))?;

        for svc in self.services.read().unwrap().iter() {
            let chars = self.discover_characteristics_by_range(svc.handle, svc.end_handle)?;
            for ch in chars {
                if ch.uuid.0 == uuid.0 {
                    return self.write_value(ch.value_handle, data, with_response);
                }
            }
        }

        Err(GattError::CharacteristicNotFound(uuid.0).into())
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state.read().unwrap().is_connected()
    }

    pub fn connection_state(&self) -> GattConnectionState {
        self.connection_state.read().unwrap().clone()
    }

    pub fn device_address(&self) -> Option<String> {
        self.device_addr.read().unwrap().clone()
    }

    pub fn get_services(&self) -> Vec<GattService> {
        self.services.read().unwrap().clone()
    }

    pub fn get_characteristics(&self) -> Vec<GattCharacteristic> {
        self.characteristics.read().unwrap().values().cloned().collect()
    }

    pub fn get_characteristic_by_handle(&self, handle: u16) -> Option<GattCharacteristic> {
        self.characteristics.read().unwrap().get(&handle).cloned()
    }

    pub fn get_characteristic_by_uuid(&self, uuid: &str) -> Option<GattCharacteristic> {
        let cleaned = uuid.replace("-", "").to_lowercase();
        self.characteristics.read().unwrap()
            .values()
            .find(|c| c.uuid.0.replace("-", "").to_lowercase() == cleaned)
            .cloned()
    }

    pub fn get_mtu(&self) -> u16 {
        *self.mtu.read().unwrap()
    }

    pub fn set_mtu(&self, mtu: u16) {
        *self.mtu.write().unwrap() = mtu;
    }

    pub fn negotiate_mtu(&self, preferred_mtu: u16) -> Result<u16, CapabilityError> {
        self.with_protocol(|proto| {
            let negotiated = proto.exchange_mtu(preferred_mtu)?;
            *self.mtu.write().unwrap() = negotiated;
            self.emit_event(GattEventKind::MTUChanged(negotiated));
            Ok(negotiated)
        })
    }

    pub fn get_socket_fd(&self) -> Option<std::os::fd::RawFd> {
        self.socket.read().unwrap().as_ref().map(|s| s.fd())
    }
}

pub struct GattClientBuilder {
    mtu: u16,
    auto_discover: bool,
    preferred_mtu: u16,
    connection_callback: Option<Box<dyn Fn(GattEventKind) + Send + Sync>>,
}

impl GattClientBuilder {
    pub fn new() -> Self {
        Self {
            mtu: 512,
            auto_discover: true,
            preferred_mtu: 512,
            connection_callback: None,
        }
    }

    pub fn mtu(mut self, mtu: u16) -> Self {
        self.mtu = mtu;
        self
    }

    pub fn auto_discover(mut self, auto: bool) -> Self {
        self.auto_discover = auto;
        self
    }

    pub fn preferred_mtu(mut self, mtu: u16) -> Self {
        self.preferred_mtu = mtu;
        self
    }

    pub fn on_event<F>(mut self, callback: F) -> Self
    where
        F: Fn(GattEventKind) + Send + Sync + 'static
    {
        self.connection_callback = Some(Box::new(callback));
        self
    }

    pub fn build(self) -> LinuxGattClient {
        let client = LinuxGattClient::with_mtu(self.mtu);
        if let Some(cb) = self.connection_callback {
            client.on_event(cb);
        }
        client
    }
}

impl Default for GattClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uuid_16bit() {
        let uuid = GattUuid::from_16(0x180D);
        let bytes = LinuxGattClient::uuid_bytes(&uuid).unwrap();
        assert_eq!(bytes, vec![0x0d, 0x18]);
    }

    #[test]
    fn parse_uuid_128bit() {
        let uuid = GattUuid("0000ff02-0000-1000-8000-00805f9b34fb".to_string());
        let bytes = LinuxGattClient::uuid_bytes(&uuid).unwrap();
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn client_constructs() {
        let client = LinuxGattClient::new();
        let desc = client.descriptor();
        assert_eq!(desc.provider_name, "linux-gatt");
    }

    #[test]
    fn parse_props() {
        let props = LinuxGattClient::parse_characteristic_properties(0x3F);
        assert!(props.contains(&GattProperty::Read));
        assert!(props.contains(&GattProperty::Write));
        assert!(props.contains(&GattProperty::Notify));
        assert!(props.contains(&GattProperty::Indicate));
    }

    #[test]
    fn builder() {
        let client = GattClientBuilder::new()
            .mtu(128)
            .auto_discover(false)
            .build();
        assert_eq!(client.get_mtu(), 128);
    }

    #[test]
    fn connection_state() {
        let client = LinuxGattClient::new();
        assert!(!client.is_connected());
        assert_eq!(client.connection_state(), GattConnectionState::Disconnected);
    }
}