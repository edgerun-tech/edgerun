#![no_std]

extern crate alloc;

#[cfg(unix)]
extern crate std;

#[cfg(not(unix))]
extern crate self as std;

#[cfg(not(unix))]
pub mod cmp {
    pub use core::cmp::*;
}

#[cfg(not(unix))]
pub mod collections {
    pub use alloc::collections::BTreeMap as HashMap;
}

#[cfg(not(unix))]
pub mod io {
    pub use edgerun_linux_sysfs::io::*;
}

#[cfg(not(unix))]
pub mod mem {
    pub use core::mem::*;
}

#[cfg(not(unix))]
pub mod os {
    pub mod fd {
        pub type RawFd = i32;

        pub trait AsRawFd {
            fn as_raw_fd(&self) -> RawFd;
        }
    }
}

#[cfg(not(unix))]
pub mod slice {
    pub use core::slice::*;
}

#[cfg(not(unix))]
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

#[cfg(not(unix))]
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

#[cfg(not(unix))]
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
pub use edgerun_protocols::bluetooth_gatt::{
    format_gatt_uuid, parse_gatt_uuid, GattAttributeType, GattProperty, GattUuid,
    UUID_CHARACTERISTIC_USER_DESCRIPTION, UUID_CLIENT_CHARACTERISTIC_CONFIGURATION,
    UUID_GATT_CHARACTERISTIC_EXTENDED_PROPERTIES, UUID_SERVER_CHARACTERISTIC_CONFIGURATION,
};

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
