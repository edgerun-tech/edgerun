#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

#[cfg(target_os = "none")]
pub mod cmp {
    pub use core::cmp::*;
}

#[cfg(target_os = "none")]
pub mod collections {
    pub use alloc::collections::BTreeMap as HashMap;
}

#[cfg(target_os = "none")]
pub mod io {
    pub use edgerun_linux_sysfs::io::*;
}

#[cfg(target_os = "none")]
pub mod mem {
    pub use core::mem::*;
}

#[cfg(target_os = "none")]
pub mod os {
    pub mod fd {
        pub type RawFd = i32;

        pub trait AsRawFd {
            fn as_raw_fd(&self) -> RawFd;
        }
    }
}

#[cfg(target_os = "none")]
pub mod slice {
    pub use core::slice::*;
}

#[cfg(target_os = "none")]
pub mod sync {
    pub use alloc::sync::Arc;
    pub mod atomic {
        pub use core::sync::atomic::*;
    }

    use core::cell::UnsafeCell;
    use core::ops::{Deref, DerefMut};

    pub struct Mutex<T>(UnsafeCell<T>);
    pub struct RwLock<T>(UnsafeCell<T>);

    unsafe impl<T: Send> Send for Mutex<T> {}
    unsafe impl<T: Send> Sync for Mutex<T> {}
    unsafe impl<T: Send + Sync> Send for RwLock<T> {}
    unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

    pub struct MutexGuard<'a, T>(&'a mut T);
    pub struct RwLockReadGuard<'a, T>(&'a T);
    pub struct RwLockWriteGuard<'a, T>(&'a mut T);

    impl<T> Mutex<T> {
        pub const fn new(value: T) -> Self {
            Self(UnsafeCell::new(value))
        }

        pub fn lock(&self) -> Result<MutexGuard<'_, T>, ()> {
            Ok(MutexGuard(unsafe { &mut *self.0.get() }))
        }
    }

    impl<T> RwLock<T> {
        pub const fn new(value: T) -> Self {
            Self(UnsafeCell::new(value))
        }

        pub fn read(&self) -> Result<RwLockReadGuard<'_, T>, ()> {
            Ok(RwLockReadGuard(unsafe { &*self.0.get() }))
        }

        pub fn write(&self) -> Result<RwLockWriteGuard<'_, T>, ()> {
            Ok(RwLockWriteGuard(unsafe { &mut *self.0.get() }))
        }
    }

    impl<T> Deref for MutexGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.0
        }
    }

    impl<T> DerefMut for MutexGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.0
        }
    }

    impl<T> Deref for RwLockReadGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.0
        }
    }

    impl<T> Deref for RwLockWriteGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.0
        }
    }

    impl<T> DerefMut for RwLockWriteGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.0
        }
    }
}

#[cfg(target_os = "none")]
pub mod thread {
    pub fn spawn<F, T>(_f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        JoinHandle(core::marker::PhantomData)
    }

    pub struct JoinHandle<T>(core::marker::PhantomData<T>);
}

#[cfg(target_os = "none")]
pub mod time {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Duration {
        millis: u128,
    }

    impl Duration {
        #[must_use]
        pub const fn from_millis(millis: u64) -> Self {
            Self {
                millis: millis as u128,
            }
        }

        #[must_use]
        pub const fn as_millis(&self) -> u128 {
            self.millis
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Instant;

    impl Instant {
        #[must_use]
        pub const fn now() -> Self {
            Self
        }

        #[must_use]
        pub const fn elapsed(&self) -> Duration {
            Duration { millis: 0 }
        }
    }
}

pub mod prelude {
    pub mod v1 {
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::cmp::Ord;
        pub use core::convert::{From, Into};
        pub use core::fmt::Write;
        pub use core::iter::{FromIterator, IntoIterator, Iterator};
        pub use core::marker::{Send, Sync};
        pub use core::matches;
        pub use core::ops::Fn;
        pub use core::option::Option::{self, None, Some};
        pub use core::prelude::rust_2024::*;
        pub use core::result::Result::{self, Err, Ok};
    }
}

use crate::prelude::v1::*;

pub mod async_ext;
mod att;
pub mod client;
pub mod error;
pub mod hci;
mod l2cap;
pub mod linux;

pub use async_ext::{AsyncAttProtocol, AsyncL2capSocket};
pub use client::LinuxGattClient;
pub use error::{GattError, GattResult};
pub use hci::{HciConnection, HciConnectionPool, LeConnParams};
pub use linux::{AttProtocol, L2capSocket};

use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};
use edgerun_encoding::byteorder::{read_u16_le, read_u32_le};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GattAddressKind {
    Public,
    Random,
    Anonymous,
    Unknown,
}

impl GattAddressKind {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0x01 => Self::Public,
            0x02 => Self::Random,
            _ => Self::Unknown,
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Public => 0x01,
            Self::Random => 0x02,
            Self::Anonymous => 0x03,
            Self::Unknown => 0x00,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GattUuid(pub String);

impl GattUuid {
    pub fn from_16(uuid: u16) -> Self {
        Self(format!("{:04x}", uuid))
    }

    pub fn as_16(&self) -> Option<u16> {
        let cleaned = self.0.replace('-', "").to_lowercase();
        if cleaned.len() == 4 {
            edgerun_encoding::hex::parse_hex_int(&cleaned)
        } else if cleaned.len() == 32 && cleaned.starts_with("0000") {
            edgerun_encoding::hex::parse_hex_int(&cleaned[4..8])
        } else {
            None
        }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let cleaned = hex.replace('-', "").to_lowercase();
        if matches!(cleaned.len(), 4 | 32) && edgerun_encoding::hex::hex_to_bytes(&cleaned).is_ok()
        {
            Some(Self(cleaned))
        } else {
            None
        }
    }

    pub fn as_128_bits(&self) -> Option<String> {
        let cleaned = self.0.replace('-', "").to_lowercase();
        if cleaned.len() == 4 {
            Some(format!("0000{}00001000800000805f9b34fb", cleaned))
        } else if cleaned.len() == 32 {
            Some(format!(
                "{}-{}-{}-{}-{}",
                &cleaned[0..8],
                &cleaned[8..12],
                &cleaned[12..16],
                &cleaned[16..20],
                &cleaned[20..32]
            ))
        } else {
            None
        }
    }

    pub fn matches_16(&self, uuid: u16) -> bool {
        if let Some(val) = self.as_16() {
            val == uuid
        } else {
            let cleaned = self.0.replace('-', "").to_lowercase();
            cleaned.ends_with(&format!("{:04x}", uuid))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GattAttributeType {
    PrimaryService,
    SecondaryService,
    Include,
    Characteristic,
    CharacteristicExtended,
    Descriptor,
    Unknown(u16),
}

impl GattAttributeType {
    pub fn from_uuid(uuid: u16) -> Self {
        match uuid {
            0x2800 => Self::PrimaryService,
            0x2801 => Self::SecondaryService,
            0x2802 => Self::Include,
            0x2803 => Self::Characteristic,
            0x2900..=0x29ff => Self::Descriptor,
            _ => Self::Unknown(uuid),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GattProperty {
    Broadcast,
    Read,
    WriteNoResponse,
    Write,
    Notify,
    Indicate,
    SignedWrite,
    Extended,
}

impl GattProperty {
    pub fn from_bits(bits: u8) -> Vec<Self> {
        let mut props = Vec::new();
        if bits & 0x01 != 0 {
            props.push(Self::Broadcast);
        }
        if bits & 0x02 != 0 {
            props.push(Self::Read);
        }
        if bits & 0x04 != 0 {
            props.push(Self::WriteNoResponse);
        }
        if bits & 0x08 != 0 {
            props.push(Self::Write);
        }
        if bits & 0x10 != 0 {
            props.push(Self::Notify);
        }
        if bits & 0x20 != 0 {
            props.push(Self::Indicate);
        }
        if bits & 0x40 != 0 {
            props.push(Self::SignedWrite);
        }
        if bits & 0x80 != 0 {
            props.push(Self::Extended);
        }
        props
    }

    pub fn has_read(&self) -> bool {
        matches!(self, Self::Read)
    }

    pub fn has_write(&self) -> bool {
        matches!(self, Self::Write | Self::WriteNoResponse)
    }

    pub fn has_notify(&self) -> bool {
        matches!(self, Self::Notify | Self::Indicate)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GattPermission {
    Read,
    ReadEncrypted,
    ReadEncryptedMitm,
    ReadEncryptedNoMitm,
    Write,
    WriteEncrypted,
    WriteEncryptedMitm,
    WriteEncryptedNoMitm,
    WriteSigned,
    WriteSignedMitm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GattService {
    pub uuid: GattUuid,
    pub primary: bool,
    pub handle: u16,
    pub end_handle: u16,
}

impl GattService {
    pub fn is_device_information(&self) -> bool {
        self.uuid.matches_16(0x180A)
    }

    pub fn is_battery_service(&self) -> bool {
        self.uuid.matches_16(0x180F)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GattCharacteristic {
    pub uuid: GattUuid,
    pub properties: Vec<GattProperty>,
    pub value_handle: u16,
    pub handle: u16,
    pub permissions: Vec<GattPermission>,
}

impl GattCharacteristic {
    pub fn has_property(&self, prop: GattProperty) -> bool {
        self.properties.contains(&prop)
    }

    pub fn isReadable(&self) -> bool {
        self.has_property(GattProperty::Read)
    }

    pub fn isWritable(&self) -> bool {
        self.has_property(GattProperty::Write) || self.has_property(GattProperty::WriteNoResponse)
    }

    pub fn isNotifiable(&self) -> bool {
        self.has_property(GattProperty::Notify) || self.has_property(GattProperty::Indicate)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GattDescriptor {
    pub uuid: GattUuid,
    pub handle: u16,
    pub permissions: Vec<GattPermission>,
}

impl GattDescriptor {
    pub fn is_client_characteristic_configuration() -> bool {
        true
    }

    pub fn is_server_characteristic_configuration() -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GattEventKind {
    ServiceDiscovered(Vec<GattService>),
    CharacteristicDiscovered(Vec<GattCharacteristic>),
    DescriptorDiscovered(Vec<GattDescriptor>),
    ValueUpdated { handle: u16, value: Vec<u8> },
    Notification { handle: u16, value: Vec<u8> },
    Indication { handle: u16, value: Vec<u8> },
    ReadResponse { handle: u16, value: Vec<u8> },
    WriteResponse { handle: u16 },
    Error { handle: u16, error: GattError },
    Disconnected,
    Connected,
    MTUChanged(u16),
    DiscoveryComplete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GattConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Encrypting,
    Encrypted,
    Disconnecting,
}

impl GattConnectionState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected | Self::Encrypting | Self::Encrypted)
    }
}

pub type GattEventCallback = Box<dyn Fn(GattEventKind) + Send + Sync>;

pub trait GattClient: CapabilityProvider {
    fn connect(
        &self,
        device_addr: &str,
        addr_type: GattAddressKind,
    ) -> Result<(), edgerun_capabilities::CapabilityError>;
    fn disconnect(&self) -> Result<(), edgerun_capabilities::CapabilityError>;
    fn discover_services(&self) -> Result<Vec<GattService>, edgerun_capabilities::CapabilityError>;
    fn discover_characteristics_by_range(
        &self,
        start: u16,
        end: u16,
    ) -> Result<Vec<GattCharacteristic>, edgerun_capabilities::CapabilityError>;
    fn discover_descriptors(
        &self,
        char_handle: u16,
    ) -> Result<Vec<GattDescriptor>, edgerun_capabilities::CapabilityError>;
    fn read_value(&self, handle: u16) -> Result<Vec<u8>, edgerun_capabilities::CapabilityError>;
    fn write_value(
        &self,
        handle: u16,
        data: &[u8],
        with_response: bool,
    ) -> Result<(), edgerun_capabilities::CapabilityError>;
    fn enable_notifications(
        &self,
        handle: u16,
        enable: bool,
    ) -> Result<(), edgerun_capabilities::CapabilityError>;
    fn read_by_type(
        &self,
        start: u16,
        end: u16,
        uuid: &GattUuid,
    ) -> Result<Vec<u8>, edgerun_capabilities::CapabilityError>;
    fn write_cmd(
        &self,
        handle: u16,
        data: &[u8],
    ) -> Result<(), edgerun_capabilities::CapabilityError>;
}

pub fn default_gatt_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Communication,
        &[CapabilityModality::Radio],
        &[CapabilityEventKind::Radio],
        &[CapabilityOperation::Observe, CapabilityOperation::Query],
        Vec::new(),
    )
}

pub fn format_gatt_uuid(bytes: &[u8]) -> String {
    match bytes.len() {
        2 => format!("{:04x}", read_u16_le(bytes, 0)),
        4 => format!("{:04x}-0000-1000-8000-00805f9b34fb", read_u32_le(bytes, 0)),
        16 => {
            let mut b = bytes.to_vec();
            b.reverse();
            let hex = b.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            format!(
                "{}-{}-{}-{}-{}",
                &hex[0..8],
                &hex[8..12],
                &hex[12..16],
                &hex[16..20],
                &hex[20..32]
            )
        }
        _ => bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>(),
    }
}

pub fn parse_gatt_uuid(uuid_str: &str) -> Option<GattUuid> {
    let cleaned = uuid_str.replace("-", "").to_lowercase();
    if cleaned.len() == 4 && cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(GattUuid(cleaned))
    } else if cleaned.len() == 8 && cleaned.starts_with("0000") {
        Some(GattUuid(cleaned[4..].to_string()))
    } else if cleaned.len() == 32 && cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(GattUuid(cleaned))
    } else {
        None
    }
}

pub const UUID_CLIENT_CHARACTERISTIC_CONFIGURATION: u16 = 0x2902;
pub const UUID_SERVER_CHARACTERISTIC_CONFIGURATION: u16 = 0x2903;
pub const UUID_CHARACTERISTIC_USER_DESCRIPTION: u16 = 0x2901;
pub const UUID_GATT_CHARACTERISTIC_EXTENDED_PROPERTIES: u16 = 0x2900;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gatt_uuid_from_16() {
        let uuid = GattUuid::from_16(0x180D);
        assert_eq!(uuid.0, "180d");
    }

    #[test]
    fn gatt_uuid_16_roundtrip() {
        let uuid = GattUuid::from_16(0xFF01);
        assert_eq!(uuid.as_16(), Some(0xFF01));
    }

    #[test]
    fn format_128bit_uuid() {
        let bytes = [
            0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80, 0x00, 0x10, 0x00, 0x00, 0x02, 0xff,
            0x00, 0x00,
        ];
        let formatted = format_gatt_uuid(&bytes);
        assert_eq!(formatted, "0000ff02-0000-1000-8000-00805f9b34fb");
    }

    #[test]
    fn parse_gatt_uuid_short() {
        let uuid = parse_gatt_uuid("ff01").unwrap();
        assert_eq!(uuid.0, "ff01");
    }

    #[test]
    fn parse_gatt_uuid_full() {
        let uuid = parse_gatt_uuid("0000ff02-0000-1000-8000-00805f9b34fb").unwrap();
        assert_eq!(uuid.0, "0000ff0200001000800000805f9b34fb");
    }

    #[test]
    fn gatt_uuid_matches_16() {
        let uuid = GattUuid::from_16(0x180A);
        assert!(uuid.matches_16(0x180A));
        let uuid2 = GattUuid::from_16(0x180A);
        assert!(uuid2.matches_16(0x180A));
    }

    #[test]
    fn gatt_property_bits() {
        let props = GattProperty::from_bits(0x1F);
        assert!(props.contains(&GattProperty::Broadcast));
        assert!(props.contains(&GattProperty::Read));
        assert!(props.contains(&GattProperty::WriteNoResponse));
        assert!(props.contains(&GattProperty::Write));
        assert!(props.contains(&GattProperty::Notify));
    }

    #[test]
    fn connection_state() {
        assert!(GattConnectionState::Disconnected.is_connected() == false);
        assert!(GattConnectionState::Connected.is_connected() == true);
        assert!(GattConnectionState::Encrypted.is_connected() == true);
    }
}
