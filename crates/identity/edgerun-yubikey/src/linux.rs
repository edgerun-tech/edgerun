#![allow(non_upper_case_globals)]

extern crate alloc;
use edgerun_encoding::byteorder::{read_u16_be, read_u16_le, read_u32_le};

#[cfg(not(unix))]
pub mod error {
    pub use core::error::*;
}

#[cfg(not(unix))]
pub mod fs {
    use crate::io;

    #[derive(Clone, Debug)]
    pub struct File;

    impl File {
        pub fn open<P>(_path: P) -> io::Result<Self> {
            Err(io::Error::last_os_error())
        }
    }

    pub struct OpenOptions;

    impl OpenOptions {
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        #[must_use]
        pub const fn read(self, _read: bool) -> Self {
            self
        }

        #[must_use]
        pub const fn write(self, _write: bool) -> Self {
            self
        }

        #[must_use]
        pub const fn custom_flags(self, _flags: i32) -> Self {
            self
        }

        pub fn open<P>(self, _path: P) -> io::Result<File> {
            Err(io::Error::last_os_error())
        }
    }

    pub use edgerun_linux_sysfs::fs::read_dir;
}

#[cfg(not(unix))]
pub mod io {
    pub use edgerun_linux_sysfs::io::*;

    pub trait Read {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    }

    pub trait Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    }

    impl Read for crate::fs::File {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
            Err(Error::last_os_error())
        }
    }

    impl Write for crate::fs::File {
        fn write_all(&mut self, _buf: &[u8]) -> Result<()> {
            Err(Error::last_os_error())
        }
    }
}

#[cfg(not(unix))]
pub mod os {
    pub mod unix {
        pub mod fs {
            pub trait OpenOptionsExt {
                fn custom_flags(self, flags: i32) -> Self;
            }

            impl OpenOptionsExt for crate::fs::OpenOptions {
                fn custom_flags(self, flags: i32) -> Self {
                    self.custom_flags(flags)
                }
            }
        }

        pub mod io {
            pub trait AsRawFd {
                fn as_raw_fd(&self) -> i32;
            }

            impl AsRawFd for crate::fs::File {
                fn as_raw_fd(&self) -> i32 {
                    -1
                }
            }
        }
    }
}

#[cfg(not(unix))]
pub mod path {
    pub use edgerun_linux_sysfs::path::*;
}

pub mod libc {
    pub const O_NONBLOCK: i32 = 0x800;

    unsafe extern "C" {
        pub fn ioctl(fd: i32, request: u64, arg: *mut core::ffi::c_void) -> i32;
    }
}

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::convert::{AsRef, Into};
use core::fmt::Write as _;
use core::iter::{IntoIterator, Iterator};
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};
use edgerun_core::crypto::signature_input;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

// YubiKey USB vendor/product IDs
const YUBIKEY_VENDOR_ID: u16 = 0x1050;
const YUBIKEY_PRODUCT_IDS: &[u16] = &[
    0x0403, 0x0404, 0x0405, 0x0406, 0x0407, 0x0408, 0x0409, 0x040a, 0x040b, 0x040c, 0x040d, 0x040e,
    0x040f, 0x0410, 0x0411, 0x0412, 0x0413, 0x0414, 0x0415, 0x0416, 0x0417, 0x0418, 0x0419, 0x041a,
    0x041b, 0x041c, 0x041d,
];

// CCID protocol constants (from USB CCID spec 1.1)
const CCID_MSG_HEADER_SIZE: usize = 10;
const CCID_PC_to_RDR_XfrBlock: u8 = 0x6F;
const CCID_RDR_to_PC_DataBlock: u8 = 0x80;
const CCID_ICC_POWER_ON: u8 = 0x62;
const CCID_ICC_POWER_OFF: u8 = 0x63;
const CCID_GET_SLOT_STATUS: u8 = 0x65;

// USB device filesystem ioctls (from linux/usbdevice_fs.h)
const USBDEVFS_RESET: u32 = 21780;
const USBDEVFS_CLAIMINTERFACE: u32 = 21770;
const USBDEVFS_RELEASEINTERFACE: u32 = 21771;
const USBDEVFS_CONTROL: u32 = 21772;
const USBDEVFS_BULK: u32 = 21773;

#[repr(C)]
struct UsbDevFsCtrl {
    brequesttype: u8,
    brequest: u8,
    wvalue: u16,
    windex: u16,
    wlength: u16,
    data: *mut u8,
    timeout: u32,
}

#[repr(C)]
struct UsbDevFsBulk {
    ep: u32,
    len: u32,
    timeout: u32,
    data: *mut u8,
}

const YUBIKEY_PIV_AID: [u8; 11] = [
    0xa0, 0x00, 0x00, 0x03, 0x08, 0x00, 0x00, 0x10, 0x00, 0x01, 0x00,
];
const YUBICO_OTP_AID: [u8; 7] = [0xa0, 0x00, 0x00, 0x05, 0x27, 0x20, 0x01];
const YUBIKEY_PIV_ATTESTATION_CERT_TAG: [u8; 3] = [0x5f, 0xff, 0x01];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum YubiKeySignatureAlgorithm {
    RsaPkcs1v15Sha256,
    RsaPssSha256,
    EcdsaP256Sha256,
    EcdsaP384Sha384,
    Eddsa,
    Opaque(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum YubiKeyAssuranceLevel {
    SoftwareSimulator,
    HardwareBacked,
    Fips,
    PivAttested,
    Certified(String),
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YubiKeyKeyInfo {
    pub slot: String,
    pub algorithm: YubiKeySignatureAlgorithm,
    pub public_key: Vec<u8>,
    pub attestation_chain: Vec<Vec<u8>>,
    pub serial_number: Option<String>,
    pub assurance_level: YubiKeyAssuranceLevel,
    pub pin_policy: Option<YubiKeyPinPolicy>,
    pub touch_policy: Option<YubiKeyTouchPolicy>,
    pub generated_on_device: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum YubiKeyError {
    Provider(String),
    UnsupportedAlgorithm(YubiKeySignatureAlgorithm),
    Pcsc(String),
    ApduStatus(u16),
    UnsupportedOperation(&'static str),
}

impl core::fmt::Display for YubiKeyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Provider(msg) => f.write_str(msg),
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(f, "unsupported YubiKey signature algorithm: {algorithm:?}")
            }
            Self::Pcsc(msg) => write!(f, "PC/SC error: {msg}"),
            Self::ApduStatus(sw) => write!(f, "YubiKey APDU status: 0x{sw:04x}"),
            Self::UnsupportedOperation(msg) => write!(f, "unsupported YubiKey operation: {msg}"),
        }
    }
}

impl core::error::Error for YubiKeyError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YubiKeyReaderInfo {
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YubiKeyPivSlot {
    Authentication,
    Signature,
    KeyManagement,
    CardAuthentication,
    Attestation,
    Retired(u8),
}

impl YubiKeyPivSlot {
    pub fn key_reference(self) -> u8 {
        match self {
            Self::Authentication => 0x9a,
            Self::Signature => 0x9c,
            Self::KeyManagement => 0x9d,
            Self::CardAuthentication => 0x9e,
            Self::Attestation => 0xf9,
            Self::Retired(index) => index,
        }
    }

    pub fn slot_name(self) -> String {
        match self {
            Self::Authentication => "9a".into(),
            Self::Signature => "9c".into(),
            Self::KeyManagement => "9d".into(),
            Self::CardAuthentication => "9e".into(),
            Self::Attestation => "f9".into(),
            Self::Retired(index) => format!("{index:02x}"),
        }
    }

    pub fn all_asymmetric_slots() -> Vec<Self> {
        let mut out = vec![
            Self::Authentication,
            Self::Signature,
            Self::KeyManagement,
            Self::CardAuthentication,
            Self::Attestation,
        ];
        out.extend((0x82..=0x95).map(Self::Retired));
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YubiKeyPinPolicy {
    Default,
    Never,
    Once,
    Always,
    MatchOnce,
    MatchAlways,
    Unknown(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YubiKeyTouchPolicy {
    Default,
    Never,
    Always,
    Cached,
    Unknown(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YubiKeyPivInfo {
    pub piv_selected: bool,
    pub yubico_otp_selected: bool,
    pub version: Option<[u8; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YubiKeyPivMetadata {
    pub slot: YubiKeyPivSlot,
    pub algorithm: Option<YubiKeySignatureAlgorithm>,
    pub pin_policy: Option<YubiKeyPinPolicy>,
    pub touch_policy: Option<YubiKeyTouchPolicy>,
    pub generated_on_device: Option<bool>,
    pub public_key: Vec<u8>,
    pub raw_tlv: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YubiKeyCapabilities {
    pub piv_applet: bool,
    pub yubico_otp_applet: bool,
    pub slot_metadata: bool,
    pub slot_attestation: bool,
    pub attestation_certificate: bool,
    pub pin_verification: bool,
    pub retired_slots: bool,
    pub touch_policy_metadata: bool,
    pub pin_policy_metadata: bool,
    pub signing: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YubiKeyApduResponse {
    pub data: Vec<u8>,
    pub status_word: u16,
}

impl YubiKeyApduResponse {
    pub fn is_success(&self) -> bool {
        self.status_word == 0x9000
    }

    pub fn into_data_if_success(self) -> Result<Vec<u8>, YubiKeyError> {
        if self.is_success() {
            Ok(self.data)
        } else {
            Err(YubiKeyError::ApduStatus(self.status_word))
        }
    }
}

// ===========================================================================
// Raw USB CCID transport — replaces PC/SC (libpcsclite)
// ===========================================================================

/// YubiKey USB device info discovered via /dev/bus/usb/
#[derive(Clone, Debug)]
pub struct LinuxUsbYubiKeyInfo {
    pub bus: u8,
    pub device: u8,
    pub product_id: u16,
    pub interface: u8,
}

/// Raw USB CCID connection to a YubiKey via /dev/bus/usb/
pub struct LinuxUsbYubiKey {
    fd: File,
    interface: u8,
    seq: u8,
}

impl Drop for LinuxUsbYubiKey {
    fn drop(&mut self) {
        let _ = self.release_interface();
    }
}

impl LinuxUsbYubiKey {
    /// Discover all YubiKey devices on the USB bus.
    pub fn discover() -> Result<Vec<LinuxUsbYubiKeyInfo>, YubiKeyError> {
        let mut results = Vec::new();
        // Scan /dev/bus/usb/BBB/DDD for YubiKey devices
        let usb_root = Path::new("/dev/bus/usb");
        if !usb_root.is_dir() {
            return Err(YubiKeyError::Provider(
                "/dev/bus/usb not found — is USB device filesystem mounted?".into(),
            ));
        }
        for bus_entry in std::fs::read_dir(usb_root)
            .map_err(|e| YubiKeyError::Provider(format!("read /dev/bus/usb: {e}")))?
        {
            let bus_entry =
                bus_entry.map_err(|e| YubiKeyError::Provider(format!("read bus dir: {e}")))?;
            let _bus_name = bus_entry.file_name();
            let bus_path = bus_entry.path();
            if !bus_path.is_dir() {
                continue;
            }
            for dev_entry in std::fs::read_dir(&bus_path)
                .map_err(|e| YubiKeyError::Provider(format!("read {bus_path:?}: {e}")))?
            {
                let dev_entry =
                    dev_entry.map_err(|e| YubiKeyError::Provider(format!("read dev dir: {e}")))?;
                let dev_path = dev_entry.path();
                if !dev_path.is_file() {
                    continue;
                }
                // Try to read device descriptor via USBDEVFS
                if let Ok(info) = Self::probe_device(&dev_path) {
                    if YUBIKEY_PRODUCT_IDS.contains(&info.product_id) {
                        results.push(info);
                    }
                }
            }
        }
        // Fallback: try known bus numbers (001-010) if enumeration failed
        if results.is_empty() {
            for bus_num in 1..=10 {
                let bus_dir = usb_root.join(format!("{bus_num:03}"));
                if let Ok(entries) = std::fs::read_dir(&bus_dir) {
                    for dev_entry in entries.flatten() {
                        let dev_path = dev_entry.path();
                        if dev_path.is_file() {
                            if let Ok(info) = Self::probe_device(&dev_path) {
                                if YUBIKEY_PRODUCT_IDS.contains(&info.product_id) {
                                    results.push(info);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    /// Open a YubiKey USB device and claim the CCID interface.
    pub fn open(info: &LinuxUsbYubiKeyInfo) -> Result<Self, YubiKeyError> {
        let dev_path = format!("/dev/bus/usb/{:03}/{:03}", info.bus, info.device);
        let mut fd = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&dev_path)
            .map_err(|e| YubiKeyError::Provider(format!("open {dev_path}: {e}")))?;

        let raw_fd = fd.as_raw_fd();

        // Reset the device to ensure clean state
        unsafe {
            let rc = libc::ioctl(raw_fd, USBDEVFS_RESET as _, core::ptr::null_mut());
            if rc < 0 {
                // Reset may fail if device is busy; continue anyway
            }
        }

        // Claim the CCID interface (usually interface 0 or 1)
        let interface = info.interface;
        let rc = unsafe {
            libc::ioctl(
                raw_fd,
                USBDEVFS_CLAIMINTERFACE as _,
                (&interface as *const u8).cast_mut().cast(),
            )
        };
        if rc < 0 {
            return Err(YubiKeyError::Provider(format!(
                "failed to claim interface {interface}: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Power on the ICC (smart card)
        Self::icc_power_on(&mut fd, interface)?;

        Ok(Self {
            fd,
            interface,
            seq: 0,
        })
    }

    /// Probe a USB device to check if it's a YubiKey.
    fn probe_device(path: &Path) -> Result<LinuxUsbYubiKeyInfo, YubiKeyError> {
        // Read device descriptor via USBDEVFS_GET_DESCRIPTOR
        // For simplicity, try to open and check the first few bytes
        let fd = File::open(path).map_err(|_| YubiKeyError::Provider("open failed".into()))?;

        // Extract bus/device numbers from path
        let path_str = path.to_string_lossy();
        let parts: Vec<&str> = path_str.split('/').collect();
        let bus: u8 = parts
            .get(parts.len() - 2)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| YubiKeyError::Provider("parse bus".into()))?;
        let device: u8 = parts
            .last()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| YubiKeyError::Provider("parse device".into()))?;

        // Try to read device descriptor via USBDEVFS
        // Device descriptor is 18 bytes, product ID is at offset 8-9 (little-endian)
        let mut desc = [0u8; 18];
        let mut ctrl = UsbDevFsCtrl {
            brequesttype: 0x80, // Device-to-host, standard, device
            brequest: 6,        // GET_DESCRIPTOR
            wvalue: 0x0100,     // Device descriptor
            windex: 0,
            wlength: 18,
            data: desc.as_mut_ptr(),
            timeout: 1000,
        };
        let rc = unsafe {
            libc::ioctl(
                fd.as_raw_fd(),
                USBDEVFS_CONTROL as _,
                (&mut ctrl as *mut UsbDevFsCtrl).cast(),
            )
        };
        if rc < 0 {
            return Err(YubiKeyError::Provider("control transfer failed".into()));
        }

        let vendor_id = read_u16_le(&desc, 8);
        let product_id = read_u16_le(&desc, 10);

        if vendor_id != YUBIKEY_VENDOR_ID {
            return Err(YubiKeyError::Provider("not a YubiKey".into()));
        }

        Ok(LinuxUsbYubiKeyInfo {
            bus,
            device,
            product_id,
            interface: 0, // CCID interface is typically 0
        })
    }

    /// Power on the ICC (smart card) via CCID.
    fn icc_power_on(fd: &mut File, interface: u8) -> Result<Vec<u8>, YubiKeyError> {
        let mut msg = vec![0u8; CCID_MSG_HEADER_SIZE];
        msg[0] = CCID_ICC_POWER_ON;
        msg[5] = interface; // bSlot

        let seq = 0u8;
        msg[6] = seq; // bSeq

        let mut buf = vec![0u8; 4096];
        Self::ccid_transfer(fd, &msg, &mut buf)
    }

    /// Release the claimed USB interface.
    fn release_interface(&self) -> Result<(), YubiKeyError> {
        let rc = unsafe {
            libc::ioctl(
                self.fd.as_raw_fd(),
                USBDEVFS_RELEASEINTERFACE as _,
                (&self.interface as *const u8).cast_mut().cast(),
            )
        };
        if rc < 0 {
            return Err(YubiKeyError::Provider(format!(
                "release interface failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }

    /// Send an APDU command and receive the response via CCID bulk transfer.
    pub fn transmit(&mut self, apdu: &[u8]) -> Result<YubiKeyApduResponse, YubiKeyError> {
        // Build CCID PC_to_RDR_XfrBlock message
        let mut msg = vec![0u8; CCID_MSG_HEADER_SIZE + apdu.len()];
        msg[0] = CCID_PC_to_RDR_XfrBlock;
        // dwLength (little-endian)
        let len_bytes = (apdu.len() as u32).to_le_bytes();
        msg[1..5].copy_from_slice(&len_bytes);
        msg[5] = self.interface; // bSlot
        self.seq = self.seq.wrapping_add(1);
        msg[6] = self.seq; // bSeq
        msg[7..].copy_from_slice(apdu);

        // Send via bulk OUT endpoint, receive via bulk IN endpoint
        let mut buf = vec![0u8; 4096];
        let response = Self::ccid_transfer(&mut self.fd, &msg, &mut buf)?;

        parse_apdu_response(&response)
    }

    /// Perform a CCID bulk transfer: write command, read response.
    fn ccid_transfer(fd: &mut File, cmd: &[u8], buf: &mut [u8]) -> Result<Vec<u8>, YubiKeyError> {
        // Write CCID command
        fd.write_all(cmd)
            .map_err(|e| YubiKeyError::Provider(format!("CCID write failed: {e}")))?;

        // Read CCID response header (10 bytes)
        let mut header = [0u8; CCID_MSG_HEADER_SIZE];
        let mut pos = 0;
        while pos < CCID_MSG_HEADER_SIZE {
            let n = fd
                .read(&mut header[pos..])
                .map_err(|e| YubiKeyError::Provider(format!("CCID read header failed: {e}")))?;
            if n == 0 {
                return Err(YubiKeyError::Provider("CCID read returned 0 bytes".into()));
            }
            pos += n;
        }

        if header[0] != CCID_RDR_to_PC_DataBlock {
            return Err(YubiKeyError::Provider(format!(
                "unexpected CCID message type: 0x{:02x}",
                header[0]
            )));
        }

        // Read data length (little-endian)
        let data_len = read_u32_le(&header, 1) as usize;
        if data_len > buf.len() {
            return Err(YubiKeyError::Provider(format!(
                "CCID response too large: {data_len}"
            )));
        }

        // Read data payload
        let mut total_read = 0;
        while total_read < data_len {
            let n = fd
                .read(&mut buf[total_read..data_len])
                .map_err(|e| YubiKeyError::Provider(format!("CCID read data failed: {e}")))?;
            if n == 0 {
                return Err(YubiKeyError::Provider(
                    "CCID data read returned 0 bytes".into(),
                ));
            }
            total_read += n;
        }

        Ok(buf[..data_len].to_vec())
    }

    /// Transmit APDU with automatic response collection (chaining).
    pub fn transmit_collect(&mut self, apdu: &[u8]) -> Result<YubiKeyApduResponse, YubiKeyError> {
        let mut response = self.transmit(apdu)?;
        let mut collected = response.data.clone();
        while (response.status_word >> 8) as u8 == 0x61 {
            let le = (response.status_word & 0xff) as u8;
            let get_response = [0x00, 0xc0, 0x00, 0x00, le];
            response = self.transmit(&get_response)?;
            collected.extend_from_slice(&response.data);
        }
        Ok(YubiKeyApduResponse {
            data: collected,
            status_word: response.status_word,
        })
    }

    /// Select the PIV applet.
    pub fn select_piv(&mut self) -> Result<YubiKeyApduResponse, YubiKeyError> {
        self.transmit_collect(&build_select_apdu(&YUBIKEY_PIV_AID))
    }

    /// Select the Yubico OTP applet.
    pub fn select_yubico_otp(&mut self) -> Result<YubiKeyApduResponse, YubiKeyError> {
        self.transmit_collect(&build_select_apdu(&YUBICO_OTP_AID))
    }

    /// Get YubiKey firmware version.
    pub fn get_yubikey_version(&mut self) -> Result<[u8; 3], YubiKeyError> {
        let resp = self.transmit_collect(&[0x00, 0xfd, 0x00, 0x00, 0x00])?;
        let data = resp.into_data_if_success()?;
        parse_yubikey_version(&data)
    }

    /// Read a data object from the PIV applet.
    pub fn get_data(&mut self, tag: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
        let resp = self.transmit_collect(&build_get_data_apdu(tag))?;
        resp.into_data_if_success()
    }

    /// Verify the PIV PIN.
    pub fn verify_pin(&mut self, pin: &[u8]) -> Result<(), YubiKeyError> {
        let resp = self.transmit_collect(&build_verify_pin_apdu(pin)?)?;
        resp.into_data_if_success().map(|_| ())
    }

    /// Get metadata for a PIV slot.
    pub fn get_piv_metadata(
        &mut self,
        slot: YubiKeyPivSlot,
    ) -> Result<YubiKeyPivMetadata, YubiKeyError> {
        let resp = self.transmit_collect(&build_get_metadata_apdu(slot))?;
        let data = resp.into_data_if_success()?;
        parse_piv_metadata(slot, &data)
    }

    /// Create an attestation statement for a PIV slot.
    pub fn create_attestation_statement(
        &mut self,
        slot: YubiKeyPivSlot,
    ) -> Result<Vec<u8>, YubiKeyError> {
        let resp = self.transmit_collect(&build_attestation_apdu(slot))?;
        resp.into_data_if_success()
    }

    /// Sign data with a PIV slot.
    pub fn sign(
        &mut self,
        slot: YubiKeyPivSlot,
        algorithm: YubiKeySignatureAlgorithm,
        message: &[u8],
    ) -> Result<Vec<u8>, YubiKeyError> {
        let digest = digest_for_yubikey_algorithm(&algorithm, message)?;
        let resp = self.transmit_collect(&build_general_authenticate_sign_apdu(
            slot, algorithm, &digest,
        )?)?;
        let data = resp.into_data_if_success()?;
        parse_general_authenticate_signature(&data)
    }

    /// Get the PIV attestation certificate.
    pub fn get_piv_attestation_certificate(&mut self) -> Result<Vec<u8>, YubiKeyError> {
        self.get_data(&YUBIKEY_PIV_ATTESTATION_CERT_TAG)
    }

    /// Probe the PIV applet status.
    pub fn probe_piv(&mut self) -> Result<YubiKeyPivInfo, YubiKeyError> {
        let piv_selected = self.select_piv()?.is_success();
        let yubico_otp_selected = self.select_yubico_otp()?.is_success();
        let version = self.get_yubikey_version().ok();
        Ok(YubiKeyPivInfo {
            piv_selected,
            yubico_otp_selected,
            version,
        })
    }
}

/// High-level YubiKey signing key using raw USB CCID transport.
/// (Backward-compatible API — discovers USB devices internally by reader name.)
pub struct LinuxPcscYubiKey {
    pub device_info: LinuxUsbYubiKeyInfo,
    pub slot: YubiKeyPivSlot,
    pub serial_number: Option<String>,
    pub pin: Option<Vec<u8>>,
}

impl LinuxPcscYubiKey {
    pub fn new(device_info: LinuxUsbYubiKeyInfo, slot: YubiKeyPivSlot) -> Self {
        Self {
            device_info,
            slot,
            serial_number: None,
            pin: None,
        }
    }

    pub fn with_serial_number(mut self, serial_number: impl Into<String>) -> Self {
        self.serial_number = Some(serial_number.into());
        self
    }

    pub fn with_pin(mut self, pin: impl AsRef<[u8]>) -> Self {
        self.pin = Some(pin.as_ref().to_vec());
        self
    }

    /// Find the USB device matching this key's device_info.
    fn find_device(&self) -> Result<LinuxUsbYubiKeyInfo, YubiKeyError> {
        Ok(self.device_info.clone())
    }

    pub fn probe(&self) -> Result<YubiKeyPivInfo, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        conn.probe_piv()
    }

    pub fn probe_capabilities(&self) -> Result<YubiKeyCapabilities, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        let info = conn.probe_piv()?;
        let metadata = conn.get_piv_metadata(self.slot).ok();
        let slot_attestation = conn.create_attestation_statement(self.slot).is_ok();
        let attestation_certificate = conn.get_piv_attestation_certificate().is_ok();
        Ok(YubiKeyCapabilities {
            piv_applet: info.piv_selected,
            yubico_otp_applet: info.yubico_otp_selected,
            slot_metadata: metadata.is_some(),
            slot_attestation,
            attestation_certificate,
            pin_verification: true,
            retired_slots: true,
            touch_policy_metadata: metadata.as_ref().and_then(|m| m.touch_policy).is_some(),
            pin_policy_metadata: metadata.as_ref().and_then(|m| m.pin_policy).is_some(),
            signing: metadata
                .as_ref()
                .and_then(|m| m.algorithm.clone())
                .is_some(),
        })
    }

    pub fn sign_with_pin(&self, pin: &[u8], message: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        conn.select_piv()?;
        conn.verify_pin(pin)?;
        let metadata = conn.get_piv_metadata(self.slot)?;
        let algorithm = metadata
            .algorithm
            .ok_or_else(|| YubiKeyError::Provider("missing slot algorithm metadata".into()))?;
        conn.sign(self.slot, algorithm, message)
    }

    pub fn sign_without_pin(&self, message: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        conn.select_piv()?;
        let metadata = conn.get_piv_metadata(self.slot)?;
        let algorithm = metadata
            .algorithm
            .ok_or_else(|| YubiKeyError::Provider("missing slot algorithm metadata".into()))?;
        conn.sign(self.slot, algorithm, message)
    }

    pub fn read_slot_metadata(&self) -> Result<YubiKeyPivMetadata, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        conn.select_piv()?;
        conn.get_piv_metadata(self.slot)
    }

    pub fn read_all_slot_metadata(&self) -> Result<Vec<YubiKeyPivMetadata>, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        conn.select_piv()?;
        let mut out = Vec::new();
        for slot in YubiKeyPivSlot::all_asymmetric_slots() {
            if let Ok(metadata) = conn.get_piv_metadata(slot) {
                out.push(metadata);
            }
        }
        Ok(out)
    }

    pub fn create_slot_attestation(&self) -> Result<Vec<u8>, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        conn.select_piv()?;
        conn.create_attestation_statement(self.slot)
    }

    pub fn read_attestation_certificate(&self) -> Result<Vec<u8>, YubiKeyError> {
        let dev = self.find_device()?;
        let mut conn = LinuxUsbYubiKey::open(&dev)?;
        conn.select_piv()?;
        conn.get_piv_attestation_certificate()
    }
}

impl YubiKeySigningKey for LinuxPcscYubiKey {
    fn key_info(&self) -> Result<YubiKeyKeyInfo, YubiKeyError> {
        let info = self.probe()?;
        let metadata = self.read_slot_metadata().ok();
        let attestation_statement = self.create_slot_attestation().ok();
        let attestation_cert = self.read_attestation_certificate().ok();
        let algorithm = metadata
            .as_ref()
            .and_then(|m| m.algorithm.clone())
            .unwrap_or_else(|| YubiKeySignatureAlgorithm::Opaque("piv-slot".into()));
        let assurance_level = if attestation_statement.is_some() || attestation_cert.is_some() {
            YubiKeyAssuranceLevel::PivAttested
        } else if info.piv_selected {
            YubiKeyAssuranceLevel::HardwareBacked
        } else {
            YubiKeyAssuranceLevel::Unknown
        };
        let public_key = metadata
            .as_ref()
            .map(|m| m.public_key.clone())
            .unwrap_or_default();
        let pin_policy = metadata.as_ref().and_then(|m| m.pin_policy);
        let touch_policy = metadata.as_ref().and_then(|m| m.touch_policy);
        let generated_on_device = metadata.as_ref().and_then(|m| m.generated_on_device);
        let mut attestation_chain = Vec::new();
        if let Some(stmt) = attestation_statement {
            attestation_chain.push(stmt);
        }
        if let Some(cert) = attestation_cert {
            attestation_chain.push(cert);
        }
        Ok(YubiKeyKeyInfo {
            slot: self.slot.slot_name(),
            algorithm,
            public_key,
            attestation_chain,
            serial_number: self.serial_number.clone(),
            assurance_level,
            pin_policy,
            touch_policy,
            generated_on_device,
        })
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
        if let Some(pin) = &self.pin {
            return self.sign_with_pin(pin, message);
        }
        self.sign_without_pin(message)
    }
}

fn build_select_apdu(aid: &[u8]) -> Vec<u8> {
    let mut apdu = Vec::with_capacity(5 + aid.len());
    apdu.extend_from_slice(&[0x00, 0xa4, 0x04, 0x00, aid.len() as u8]);
    apdu.extend_from_slice(aid);
    apdu
}

fn build_get_metadata_apdu(slot: YubiKeyPivSlot) -> [u8; 4] {
    [0x00, 0xf7, 0x00, slot.key_reference()]
}

fn build_attestation_apdu(slot: YubiKeyPivSlot) -> [u8; 4] {
    [0x00, 0xf9, slot.key_reference(), 0x00]
}

fn build_get_data_apdu(tag: &[u8]) -> Vec<u8> {
    let mut data_field = Vec::with_capacity(2 + tag.len());
    data_field.push(0x5c);
    data_field.push(tag.len() as u8);
    data_field.extend_from_slice(tag);
    let mut apdu = Vec::with_capacity(5 + data_field.len());
    apdu.extend_from_slice(&[0x00, 0xcb, 0x3f, 0xff, data_field.len() as u8]);
    apdu.extend_from_slice(&data_field);
    apdu
}

fn build_verify_pin_apdu(pin: &[u8]) -> Result<[u8; 13], YubiKeyError> {
    if pin.len() < 6 || pin.len() > 8 {
        return Err(YubiKeyError::Provider(
            "PIV PIN must be between 6 and 8 bytes".into(),
        ));
    }
    let mut apdu = [0xffu8; 13];
    apdu[0..5].copy_from_slice(&[0x00, 0x20, 0x00, 0x80, 0x08]);
    apdu[5..5 + pin.len()].copy_from_slice(pin);
    Ok(apdu)
}

fn yubikey_piv_algorithm_id(algorithm: &YubiKeySignatureAlgorithm) -> Option<u8> {
    match algorithm {
        YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256 => Some(0x07),
        YubiKeySignatureAlgorithm::RsaPssSha256 => Some(0x07),
        YubiKeySignatureAlgorithm::EcdsaP256Sha256 => Some(0x11),
        YubiKeySignatureAlgorithm::EcdsaP384Sha384 => Some(0x14),
        _ => None,
    }
}

fn digest_for_yubikey_algorithm(
    algorithm: &YubiKeySignatureAlgorithm,
    message: &[u8],
) -> Result<Vec<u8>, YubiKeyError> {
    match algorithm {
        YubiKeySignatureAlgorithm::EcdsaP256Sha256
        | YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256
        | YubiKeySignatureAlgorithm::RsaPssSha256 => {
            Ok(edgerun_core::crypto::sha256(message).to_vec())
        }
        YubiKeySignatureAlgorithm::EcdsaP384Sha384 => Ok(edgerun_core::crypto::sha384(message)),
        other => Err(YubiKeyError::UnsupportedAlgorithm(other.clone())),
    }
}

fn encode_tlv(tag: u8, value: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
    edgerun_encoding::tlv::encode_tlv(tag, value)
        .map_err(|e| YubiKeyError::Provider(format!("TLV encode failed: {e:?}")))
}

fn build_general_authenticate_sign_apdu(
    slot: YubiKeyPivSlot,
    algorithm: YubiKeySignatureAlgorithm,
    digest: &[u8],
) -> Result<Vec<u8>, YubiKeyError> {
    let alg = yubikey_piv_algorithm_id(&algorithm)
        .ok_or_else(|| YubiKeyError::UnsupportedAlgorithm(algorithm.clone()))?;
    let witness = encode_tlv(0x82, &[])?;
    let challenge = encode_tlv(0x81, digest)?;
    let mut dynamic = encode_tlv(0x7c, &[witness, challenge].concat())?;
    let mut apdu = Vec::with_capacity(5 + dynamic.len());
    apdu.extend_from_slice(&[0x00, 0x87, alg, slot.key_reference(), dynamic.len() as u8]);
    apdu.append(&mut dynamic);
    Ok(apdu)
}

fn parse_apdu_response(bytes: &[u8]) -> Result<YubiKeyApduResponse, YubiKeyError> {
    if bytes.len() < 2 {
        return Err(YubiKeyError::Provider("short APDU response".into()));
    }
    let split = bytes.len() - 2;
    Ok(YubiKeyApduResponse {
        data: bytes[..split].to_vec(),
        status_word: read_u16_be(bytes, split),
    })
}

fn parse_pcsc_multi_string(bytes: &[u8]) -> Vec<String> {
    edgerun_encoding::cstring::decode_c_multi_string_lossy_until_empty(bytes)
}

fn parse_yubikey_version(bytes: &[u8]) -> Result<[u8; 3], YubiKeyError> {
    if bytes.len() < 3 {
        return Err(YubiKeyError::Provider(
            "short YubiKey version response".into(),
        ));
    }
    Ok([bytes[0], bytes[1], bytes[2]])
}

fn parse_general_authenticate_signature(bytes: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
    let outer = parse_tlv_map(bytes)?;
    for (tag, value) in outer {
        if tag != 0x7c {
            continue;
        }
        let entries = parse_tlv_map(&value)?;
        for (inner_tag, inner_value) in entries {
            if inner_tag == 0x82 {
                return Ok(inner_value);
            }
        }
    }
    Err(YubiKeyError::Provider(
        "GENERAL AUTHENTICATE response missing signature".into(),
    ))
}

fn parse_tlv_map(bytes: &[u8]) -> Result<Vec<(u8, Vec<u8>)>, YubiKeyError> {
    edgerun_encoding::tlv::parse_tlv_map(bytes)
        .map_err(|e| YubiKeyError::Provider(format!("TLV parse failed: {e:?}")))
}

fn parse_piv_algorithm(id: u8) -> Option<YubiKeySignatureAlgorithm> {
    match id {
        0x06 | 0x07 | 0x05 | 0x16 => Some(YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256),
        0x11 => Some(YubiKeySignatureAlgorithm::EcdsaP256Sha256),
        0x14 => Some(YubiKeySignatureAlgorithm::EcdsaP384Sha384),
        _ => None,
    }
}

fn parse_pin_policy(value: u8) -> YubiKeyPinPolicy {
    match value {
        0x00 => YubiKeyPinPolicy::Default,
        0x01 => YubiKeyPinPolicy::Never,
        0x02 => YubiKeyPinPolicy::Once,
        0x03 => YubiKeyPinPolicy::Always,
        0x04 => YubiKeyPinPolicy::MatchOnce,
        0x05 => YubiKeyPinPolicy::MatchAlways,
        other => YubiKeyPinPolicy::Unknown(other),
    }
}

fn parse_touch_policy(value: u8) -> YubiKeyTouchPolicy {
    match value {
        0x00 => YubiKeyTouchPolicy::Default,
        0x01 => YubiKeyTouchPolicy::Never,
        0x02 => YubiKeyTouchPolicy::Always,
        0x03 => YubiKeyTouchPolicy::Cached,
        other => YubiKeyTouchPolicy::Unknown(other),
    }
}

fn parse_piv_metadata(
    slot: YubiKeyPivSlot,
    bytes: &[u8],
) -> Result<YubiKeyPivMetadata, YubiKeyError> {
    if bytes.is_empty() {
        return Err(YubiKeyError::Provider("empty metadata response".into()));
    }
    let body = if bytes[0] == 0x53 { &bytes[1..] } else { bytes };
    let entries = parse_tlv_map(body)?;
    let mut algorithm = None;
    let mut pin_policy = None;
    let mut touch_policy = None;
    let mut generated_on_device = None;
    let mut public_key = Vec::new();
    for (tag, value) in entries {
        match tag {
            0x01 if !value.is_empty() => algorithm = parse_piv_algorithm(value[0]),
            0x02 if value.len() >= 2 => {
                pin_policy = Some(parse_pin_policy(value[0]));
                touch_policy = Some(parse_touch_policy(value[1]));
            }
            0x03 if !value.is_empty() => generated_on_device = Some(value[0] != 0),
            0x04 => public_key = value,
            _ => {}
        }
    }
    Ok(YubiKeyPivMetadata {
        slot,
        algorithm,
        pin_policy,
        touch_policy,
        generated_on_device,
        public_key,
        raw_tlv: bytes.to_vec(),
    })
}

pub(crate) fn list_pcsc_readers() -> Result<Vec<YubiKeyReaderInfo>, YubiKeyError> {
    LinuxUsbYubiKey::discover().map(|devices| {
        devices
            .into_iter()
            .map(|info| YubiKeyReaderInfo {
                name: format!("USB:{:03}:{:03}", info.bus, info.device),
            })
            .collect()
    })
}

#[cfg(test)]
pub(crate) fn looks_like_yubikey_reader(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    lowered.contains("yubikey") || lowered.contains("yubi")
}

pub trait YubiKeySigningKey {
    fn key_info(&self) -> Result<YubiKeyKeyInfo, YubiKeyError>;

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, YubiKeyError>;
}

pub(crate) fn signature_input_for_record(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    signature_input(sig_domain_tag, record_hash)
}

pub(crate) fn sign_record_with_yubikey(
    key: &dyn YubiKeySigningKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, YubiKeyError> {
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

pub fn sign_record_with_yubikey_checked(
    key: &dyn YubiKeySigningKey,
    expected_algorithms: &[YubiKeySignatureAlgorithm],
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, YubiKeyError> {
    let key_info = key.key_info()?;
    if !expected_algorithms
        .iter()
        .any(|algorithm| algorithm == &key_info.algorithm)
    {
        return Err(YubiKeyError::UnsupportedAlgorithm(key_info.algorithm));
    }
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    struct FakeYubiKey {
        algorithm: YubiKeySignatureAlgorithm,
    }

    impl FakeYubiKey {
        fn new(algorithm: YubiKeySignatureAlgorithm) -> Self {
            Self { algorithm }
        }
    }

    impl YubiKeySigningKey for FakeYubiKey {
        fn key_info(&self) -> Result<YubiKeyKeyInfo, YubiKeyError> {
            Ok(YubiKeyKeyInfo {
                slot: "9c".into(),
                algorithm: self.algorithm.clone(),
                public_key: vec![8, 6, 7, 5, 3, 0, 9],
                attestation_chain: vec![vec![1, 2, 3]],
                serial_number: Some("yk-serial-123".into()),
                assurance_level: YubiKeyAssuranceLevel::PivAttested,
                pin_policy: Some(YubiKeyPinPolicy::Always),
                touch_policy: Some(YubiKeyTouchPolicy::Always),
                generated_on_device: Some(true),
            })
        }

        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
            let mut sig = self.key_info()?.public_key;
            sig.extend_from_slice(message);
            Ok(sig)
        }
    }

    #[test]
    fn signs_protocol_record_input_via_generic_yubikey_trait() {
        let key = FakeYubiKey::new(YubiKeySignatureAlgorithm::EcdsaP256Sha256);
        let sig = sign_record_with_yubikey(&key, "edgerun:v0:sig:test", &[4u8; 32]).unwrap();
        let expected_input = signature_input_for_record("edgerun:v0:sig:test", &[4u8; 32]);
        assert!(sig.ends_with(&expected_input));
    }

    #[test]
    fn checked_sign_rejects_unexpected_algorithm() {
        let key = FakeYubiKey::new(YubiKeySignatureAlgorithm::RsaPssSha256);
        let err = sign_record_with_yubikey_checked(
            &key,
            &[
                YubiKeySignatureAlgorithm::EcdsaP256Sha256,
                YubiKeySignatureAlgorithm::Eddsa,
            ],
            "edgerun:v0:sig:test",
            &[4u8; 32],
        )
        .unwrap_err();
        assert_eq!(
            err,
            YubiKeyError::UnsupportedAlgorithm(YubiKeySignatureAlgorithm::RsaPssSha256)
        );
    }

    #[test]
    fn checked_sign_accepts_expected_algorithm() {
        let key = FakeYubiKey::new(YubiKeySignatureAlgorithm::Eddsa);
        let sig = sign_record_with_yubikey_checked(
            &key,
            &[
                YubiKeySignatureAlgorithm::EcdsaP256Sha256,
                YubiKeySignatureAlgorithm::Eddsa,
            ],
            "edgerun:v0:sig:test",
            &[4u8; 32],
        )
        .unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn build_select_apdu_uses_expected_shape() {
        let apdu = build_select_apdu(&YUBIKEY_PIV_AID);
        assert_eq!(apdu[0..4], [0x00, 0xa4, 0x04, 0x00]);
        assert_eq!(apdu[4] as usize, YUBIKEY_PIV_AID.len());
        assert_eq!(&apdu[5..], &YUBIKEY_PIV_AID);
    }

    #[test]
    fn parse_apdu_response_works() {
        let resp = parse_apdu_response(&[0x01, 0x02, 0x90, 0x00]).unwrap();
        assert_eq!(resp.data, vec![0x01, 0x02]);
        assert_eq!(resp.status_word, 0x9000);
        assert!(resp.is_success());
    }

    #[test]
    fn parse_pcsc_multi_string_works() {
        let names = parse_pcsc_multi_string(b"YubiKey 5 NFC\0Other Reader\0\0");
        assert_eq!(names, vec!["YubiKey 5 NFC", "Other Reader"]);
    }

    #[test]
    fn looks_like_yubikey_reader_matches_common_names() {
        assert!(looks_like_yubikey_reader("YubiKey 5 NFC"));
        assert!(looks_like_yubikey_reader("Yubico YubiKey OTP+FIDO+CCID"));
        assert!(!looks_like_yubikey_reader("ACS ACR39U"));
    }

    #[test]
    fn parse_yubikey_version_requires_three_bytes() {
        assert_eq!(parse_yubikey_version(&[5, 7, 1]).unwrap(), [5, 7, 1]);
        assert!(parse_yubikey_version(&[5, 7]).is_err());
    }

    #[test]
    fn build_get_metadata_apdu_uses_expected_shape() {
        let apdu = build_get_metadata_apdu(YubiKeyPivSlot::Signature);
        assert_eq!(apdu, [0x00, 0xf7, 0x00, 0x9c]);
    }

    #[test]
    fn build_attestation_apdu_uses_expected_shape() {
        let apdu = build_attestation_apdu(YubiKeyPivSlot::Signature);
        assert_eq!(apdu, [0x00, 0xf9, 0x9c, 0x00]);
    }

    #[test]
    fn build_get_data_apdu_wraps_tag_in_5c_tlv() {
        let apdu = build_get_data_apdu(&YUBIKEY_PIV_ATTESTATION_CERT_TAG);
        assert_eq!(apdu[0..4], [0x00, 0xcb, 0x3f, 0xff]);
        assert_eq!(apdu[5..], [0x5c, 0x03, 0x5f, 0xff, 0x01]);
    }

    #[test]
    fn build_verify_pin_apdu_pads_with_ff() {
        let apdu = build_verify_pin_apdu(b"123456").unwrap();
        assert_eq!(apdu[0..5], [0x00, 0x20, 0x00, 0x80, 0x08]);
        assert_eq!(&apdu[5..11], b"123456");
        assert_eq!(apdu[11], 0xff);
        assert_eq!(apdu[12], 0xff);
    }

    #[test]
    fn build_verify_pin_apdu_rejects_invalid_lengths() {
        assert!(build_verify_pin_apdu(b"12345").is_err());
        assert!(build_verify_pin_apdu(b"123456789").is_err());
    }

    #[test]
    fn digest_for_yubikey_algorithm_hashes_expected_lengths() {
        assert_eq!(
            digest_for_yubikey_algorithm(&YubiKeySignatureAlgorithm::EcdsaP256Sha256, b"abc")
                .unwrap()
                .len(),
            32
        );
        assert_eq!(
            digest_for_yubikey_algorithm(&YubiKeySignatureAlgorithm::EcdsaP384Sha384, b"abc")
                .unwrap()
                .len(),
            48
        );
    }

    #[test]
    fn build_general_authenticate_sign_apdu_wraps_digest() {
        let apdu = build_general_authenticate_sign_apdu(
            YubiKeyPivSlot::Signature,
            YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            &[0xaa, 0xbb, 0xcc],
        )
        .unwrap();
        assert_eq!(apdu[0..4], [0x00, 0x87, 0x11, 0x9c]);
        assert_eq!(apdu[5], 0x7c);
    }

    #[test]
    fn parse_tlv_map_works_for_short_values() {
        let parsed = parse_tlv_map(&[0x01, 0x01, 0x11, 0x03, 0x01, 0x01]).unwrap();
        assert_eq!(parsed, vec![(0x01, vec![0x11]), (0x03, vec![0x01])]);
    }

    #[test]
    fn parse_piv_metadata_extracts_algorithm_policy_origin_and_key() {
        let metadata = parse_piv_metadata(
            YubiKeyPivSlot::Signature,
            &[
                0x53, 0x01, 0x01, 0x11, 0x02, 0x02, 0x03, 0x02, 0x03, 0x01, 0x01, 0x04, 0x03, 0xaa,
                0xbb, 0xcc,
            ],
        )
        .unwrap();
        assert_eq!(
            metadata.algorithm,
            Some(YubiKeySignatureAlgorithm::EcdsaP256Sha256)
        );
        assert_eq!(metadata.pin_policy, Some(YubiKeyPinPolicy::Always));
        assert_eq!(metadata.touch_policy, Some(YubiKeyTouchPolicy::Always));
        assert_eq!(metadata.generated_on_device, Some(true));
        assert_eq!(metadata.public_key, vec![0xaa, 0xbb, 0xcc]);
    }

    #[test]
    fn parse_policy_values_cover_expected_variants() {
        assert_eq!(parse_pin_policy(0x03), YubiKeyPinPolicy::Always);
        assert_eq!(parse_touch_policy(0x03), YubiKeyTouchPolicy::Cached);
        assert_eq!(parse_pin_policy(0xff), YubiKeyPinPolicy::Unknown(0xff));
        assert_eq!(parse_touch_policy(0xff), YubiKeyTouchPolicy::Unknown(0xff));
    }

    #[test]
    fn parse_general_authenticate_signature_extracts_inner_82_value() {
        let sig = parse_general_authenticate_signature(&[0x7c, 0x05, 0x82, 0x03, 0x11, 0x22, 0x33])
            .unwrap();
        assert_eq!(sig, vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn apdu_response_into_data_if_success_rejects_failure_status() {
        let err = YubiKeyApduResponse {
            data: vec![],
            status_word: 0x6a82,
        }
        .into_data_if_success()
        .unwrap_err();
        assert_eq!(err, YubiKeyError::ApduStatus(0x6a82));
    }

    #[test]
    fn linux_pcsc_yubikey_builder_methods_set_fields() {
        let device_info = LinuxUsbYubiKeyInfo {
            bus: 1,
            device: 5,
            product_id: 0x0407,
            interface: 0,
        };
        let key = LinuxPcscYubiKey::new(device_info, YubiKeyPivSlot::Signature)
            .with_serial_number("123456")
            .with_pin("123456");
        assert_eq!(key.serial_number, Some("123456".into()));
        assert_eq!(key.pin, Some(b"123456".to_vec()));
        assert_eq!(key.device_info.bus, 1);
        assert_eq!(key.slot, YubiKeyPivSlot::Signature);
    }

    #[test]
    fn slot_helpers_cover_retired_and_attestation_slots() {
        assert_eq!(YubiKeyPivSlot::Attestation.key_reference(), 0xf9);
        assert_eq!(YubiKeyPivSlot::Attestation.slot_name(), "f9");
        assert_eq!(YubiKeyPivSlot::Retired(0x82).slot_name(), "82");
        assert_eq!(YubiKeyPivSlot::all_asymmetric_slots().len(), 25);
    }

    #[test]
    fn capabilities_struct_tracks_signing_and_policy_support() {
        let caps = YubiKeyCapabilities {
            piv_applet: true,
            yubico_otp_applet: true,
            slot_metadata: true,
            slot_attestation: true,
            attestation_certificate: true,
            pin_verification: true,
            retired_slots: true,
            touch_policy_metadata: true,
            pin_policy_metadata: true,
            signing: true,
        };
        assert!(caps.signing);
        assert!(caps.touch_policy_metadata);
        assert!(caps.pin_policy_metadata);
    }

    // --- Additional enum variant coverage ---

    #[test]
    fn signature_algorithm_variants_are_distinct() {
        let algorithms = vec![
            YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256,
            YubiKeySignatureAlgorithm::RsaPssSha256,
            YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            YubiKeySignatureAlgorithm::EcdsaP384Sha384,
            YubiKeySignatureAlgorithm::Eddsa,
            YubiKeySignatureAlgorithm::Opaque("custom".into()),
        ];
        for (i, a) in algorithms.iter().enumerate() {
            for (j, b) in algorithms.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn assurance_level_variants_are_distinct() {
        let levels = vec![
            YubiKeyAssuranceLevel::SoftwareSimulator,
            YubiKeyAssuranceLevel::HardwareBacked,
            YubiKeyAssuranceLevel::Fips,
            YubiKeyAssuranceLevel::PivAttested,
            YubiKeyAssuranceLevel::Certified("chain".into()),
            YubiKeyAssuranceLevel::Unknown,
        ];
        for (i, a) in levels.iter().enumerate() {
            for (j, b) in levels.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn pin_policy_variants_are_distinct() {
        let policies = vec![
            YubiKeyPinPolicy::Default,
            YubiKeyPinPolicy::Never,
            YubiKeyPinPolicy::Once,
            YubiKeyPinPolicy::Always,
            YubiKeyPinPolicy::MatchOnce,
            YubiKeyPinPolicy::MatchAlways,
            YubiKeyPinPolicy::Unknown(0xFF),
        ];
        for (i, a) in policies.iter().enumerate() {
            for (j, b) in policies.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn touch_policy_variants_are_distinct() {
        let policies = vec![
            YubiKeyTouchPolicy::Default,
            YubiKeyTouchPolicy::Never,
            YubiKeyTouchPolicy::Always,
            YubiKeyTouchPolicy::Cached,
            YubiKeyTouchPolicy::Unknown(0xFF),
        ];
        for (i, a) in policies.iter().enumerate() {
            for (j, b) in policies.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn piv_slot_variants_are_distinct() {
        let slots = vec![
            YubiKeyPivSlot::Authentication,
            YubiKeyPivSlot::Signature,
            YubiKeyPivSlot::KeyManagement,
            YubiKeyPivSlot::CardAuthentication,
            YubiKeyPivSlot::Attestation,
            YubiKeyPivSlot::Retired(0x82),
            YubiKeyPivSlot::Retired(0x95),
        ];
        for (i, a) in slots.iter().enumerate() {
            for (j, b) in slots.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // --- YubiKeyError Display and Debug ---

    #[test]
    fn error_display_provider() {
        let err = YubiKeyError::Provider("failed".into());
        assert_eq!(format!("{}", err), "failed");
    }

    #[test]
    fn error_display_unsupported_algorithm() {
        let err = YubiKeyError::UnsupportedAlgorithm(YubiKeySignatureAlgorithm::Eddsa);
        let msg = format!("{}", err);
        assert!(msg.contains("unsupported"));
        assert!(msg.contains("Eddsa"));
    }

    #[test]
    fn error_display_pcsc() {
        let err = YubiKeyError::Pcsc("SCardConnect failed".into());
        let msg = format!("{}", err);
        assert!(msg.contains("PC/SC"));
        assert!(msg.contains("SCardConnect failed"));
    }

    #[test]
    fn error_display_apdu_status() {
        let err = YubiKeyError::ApduStatus(0x6A82);
        let msg = format!("{}", err);
        assert!(msg.contains("6a82"));
    }

    #[test]
    fn error_display_unsupported_operation() {
        let err = YubiKeyError::UnsupportedOperation("attestation");
        let msg = format!("{}", err);
        assert!(msg.contains("unsupported"));
        assert!(msg.contains("attestation"));
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(YubiKeyError::Provider("err".into()));
        assert!(err.to_string().contains("err"));
    }

    #[test]
    fn error_debug_contains_variant_name() {
        let err = YubiKeyError::ApduStatus(0x9000);
        let debug = format!("{:?}", err);
        assert!(debug.contains("ApduStatus"));
    }

    // --- YubiKeyApduResponse ---

    #[test]
    fn apdu_response_is_success_false_for_non_9000() {
        let resp = YubiKeyApduResponse {
            data: vec![],
            status_word: 0x9001,
        };
        assert!(!resp.is_success());
    }

    #[test]
    fn apdu_response_into_data_if_success_returns_data_on_success() {
        let resp = YubiKeyApduResponse {
            data: vec![1, 2, 3],
            status_word: 0x9000,
        };
        assert_eq!(resp.into_data_if_success().unwrap(), vec![1, 2, 3]);
    }

    // --- TLV encoding ---

    #[test]
    fn encode_tlv_short_value_single_length_byte() {
        let encoded = encode_tlv(0x01, &[0xAA, 0xBB]).unwrap();
        assert_eq!(encoded, vec![0x01, 0x02, 0xAA, 0xBB]);
    }

    #[test]
    fn encode_tlv_empty_value() {
        let encoded = encode_tlv(0x82, &[]).unwrap();
        assert_eq!(encoded, vec![0x82, 0x00]);
    }

    #[test]
    fn encode_tlv_127_bytes_single_length_byte() {
        let value = vec![0xAA; 127];
        let encoded = encode_tlv(0x01, &value).unwrap();
        assert_eq!(encoded[0], 0x01);
        assert_eq!(encoded[1], 127);
        assert_eq!(encoded.len(), 2 + 127);
    }

    #[test]
    fn encode_tlv_128_bytes_two_length_bytes() {
        let value = vec![0xAA; 128];
        let encoded = encode_tlv(0x01, &value).unwrap();
        assert_eq!(encoded[0], 0x01);
        assert_eq!(encoded[1..3], [0x81, 128]);
        assert_eq!(encoded.len(), 3 + 128);
    }

    #[test]
    fn encode_tlv_255_bytes_single_length_byte_max() {
        let value = vec![0xAA; 255];
        let encoded = encode_tlv(0x01, &value).unwrap();
        assert_eq!(encoded[0], 0x01);
        assert_eq!(encoded[1..3], [0x81, 255]);
        assert_eq!(encoded.len(), 3 + 255);
    }

    #[test]
    fn encode_tlv_65536_bytes_is_too_long() {
        let value = vec![0xAA; 65536];
        let err = encode_tlv(0x01, &value).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    // --- parse_tlv_map edge cases ---

    #[test]
    fn parse_tlv_map_empty_input() {
        let parsed = parse_tlv_map(&[]).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_tlv_map_short_header() {
        let err = parse_tlv_map(&[0x01]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    #[test]
    fn parse_tlv_map_short_value() {
        let err = parse_tlv_map(&[0x01, 0x05, 0xAA]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    #[test]
    fn parse_tlv_map_extended_length() {
        let parsed = parse_tlv_map(&[0x01, 0x81, 0x03, 0xAA, 0xBB, 0xCC]).unwrap();
        assert_eq!(parsed, vec![(0x01, vec![0xAA, 0xBB, 0xCC])]);
    }

    #[test]
    fn parse_tlv_map_unsupported_length_encoding() {
        // 0x83 means 3-byte length, which we don't support
        let err = parse_tlv_map(&[0x01, 0x83, 0x00, 0x00, 0x01]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    // --- parse_piv_algorithm ---

    #[test]
    fn parse_piv_algorithm_rsa_variants() {
        for &id in &[0x06, 0x07, 0x05, 0x16] {
            let alg = parse_piv_algorithm(id);
            assert_eq!(
                alg,
                Some(YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256),
                "id 0x{id:02x}"
            );
        }
    }

    #[test]
    fn parse_piv_algorithm_ec_variants() {
        assert_eq!(
            parse_piv_algorithm(0x11),
            Some(YubiKeySignatureAlgorithm::EcdsaP256Sha256)
        );
        assert_eq!(
            parse_piv_algorithm(0x14),
            Some(YubiKeySignatureAlgorithm::EcdsaP384Sha384)
        );
    }

    #[test]
    fn parse_piv_algorithm_unknown_returns_none() {
        assert_eq!(parse_piv_algorithm(0xFF), None);
        assert_eq!(parse_piv_algorithm(0x99), None);
    }

    // --- digest_for_yubikey_algorithm ---

    #[test]
    fn digest_for_rsa_pkcs1_sha256() {
        let d =
            digest_for_yubikey_algorithm(&YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256, b"test")
                .unwrap();
        assert_eq!(d.len(), 32);
    }

    #[test]
    fn digest_for_rsa_pss_sha256() {
        let d = digest_for_yubikey_algorithm(&YubiKeySignatureAlgorithm::RsaPssSha256, b"test")
            .unwrap();
        assert_eq!(d.len(), 32);
    }

    #[test]
    fn digest_for_eddsa_is_unsupported() {
        let err =
            digest_for_yubikey_algorithm(&YubiKeySignatureAlgorithm::Eddsa, b"test").unwrap_err();
        assert!(matches!(err, YubiKeyError::UnsupportedAlgorithm(_)));
    }

    #[test]
    fn digest_for_opaque_is_unsupported() {
        let err = digest_for_yubikey_algorithm(
            &YubiKeySignatureAlgorithm::Opaque("custom".into()),
            b"test",
        )
        .unwrap_err();
        assert!(matches!(err, YubiKeyError::UnsupportedAlgorithm(_)));
    }

    // --- yubikey_piv_algorithm_id ---

    #[test]
    fn algorithm_id_rsa_pkcs1() {
        assert_eq!(
            yubikey_piv_algorithm_id(&YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256),
            Some(0x07)
        );
    }

    #[test]
    fn algorithm_id_rsa_pss() {
        assert_eq!(
            yubikey_piv_algorithm_id(&YubiKeySignatureAlgorithm::RsaPssSha256),
            Some(0x07)
        );
    }

    #[test]
    fn algorithm_id_ecdsa_p256() {
        assert_eq!(
            yubikey_piv_algorithm_id(&YubiKeySignatureAlgorithm::EcdsaP256Sha256),
            Some(0x11)
        );
    }

    #[test]
    fn algorithm_id_ecdsa_p384() {
        assert_eq!(
            yubikey_piv_algorithm_id(&YubiKeySignatureAlgorithm::EcdsaP384Sha384),
            Some(0x14)
        );
    }

    #[test]
    fn algorithm_id_eddsa_is_none() {
        assert_eq!(
            yubikey_piv_algorithm_id(&YubiKeySignatureAlgorithm::Eddsa),
            None
        );
    }

    #[test]
    fn algorithm_id_opaque_is_none() {
        assert_eq!(
            yubikey_piv_algorithm_id(&YubiKeySignatureAlgorithm::Opaque("x".into())),
            None
        );
    }

    // --- parse_piv_metadata edge cases ---

    #[test]
    fn parse_piv_metadata_empty_input() {
        let err = parse_piv_metadata(YubiKeyPivSlot::Signature, &[]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    #[test]
    fn parse_piv_metadata_without_53_wrapper() {
        let metadata =
            parse_piv_metadata(YubiKeyPivSlot::Authentication, &[0x01, 0x01, 0x11]).unwrap();
        assert_eq!(
            metadata.algorithm,
            Some(YubiKeySignatureAlgorithm::EcdsaP256Sha256)
        );
        assert_eq!(metadata.slot, YubiKeyPivSlot::Authentication);
    }

    #[test]
    fn parse_piv_metadata_minimal() {
        // 0x53 tag with 1 byte of content: a zero-length inner TLV (0x01, 0x00)
        let metadata =
            parse_piv_metadata(YubiKeyPivSlot::Attestation, &[0x53, 0x02, 0x01, 0x00]).unwrap();
        assert_eq!(metadata.algorithm, None);
        assert_eq!(metadata.pin_policy, None);
        assert_eq!(metadata.touch_policy, None);
        assert_eq!(metadata.generated_on_device, None);
        assert!(metadata.public_key.is_empty());
    }

    // --- parse_general_authenticate_signature edge cases ---

    #[test]
    fn parse_general_authenticate_signature_no_7c_wrapper() {
        let err =
            parse_general_authenticate_signature(&[0x82, 0x03, 0x11, 0x22, 0x33]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    #[test]
    fn parse_general_authenticate_signature_7c_no_82_inner() {
        let err = parse_general_authenticate_signature(&[0x7c, 0x02, 0x81, 0x01]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    // --- build_general_authenticate_sign_apdu ---

    #[test]
    fn build_sign_apdu_unsupported_algorithm() {
        let err = build_general_authenticate_sign_apdu(
            YubiKeyPivSlot::Signature,
            YubiKeySignatureAlgorithm::Eddsa,
            &[0xAA; 32],
        )
        .unwrap_err();
        assert!(matches!(err, YubiKeyError::UnsupportedAlgorithm(_)));
    }

    #[test]
    fn build_sign_apdu_ecdsa_p384() {
        let apdu = build_general_authenticate_sign_apdu(
            YubiKeyPivSlot::KeyManagement,
            YubiKeySignatureAlgorithm::EcdsaP384Sha384,
            &[0xBB; 48],
        )
        .unwrap();
        assert_eq!(apdu[0..4], [0x00, 0x87, 0x14, 0x9d]);
    }

    // --- parse_apdu_response edge cases ---

    #[test]
    fn parse_apdu_response_too_short() {
        let err = parse_apdu_response(&[0x90]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    #[test]
    fn parse_apdu_response_empty() {
        let err = parse_apdu_response(&[]).unwrap_err();
        assert!(matches!(err, YubiKeyError::Provider(_)));
    }

    #[test]
    fn parse_apdu_response_only_status_word() {
        let resp = parse_apdu_response(&[0x69, 0x85]).unwrap();
        assert!(resp.data.is_empty());
        assert_eq!(resp.status_word, 0x6985);
        assert!(!resp.is_success());
    }

    // --- parse_pcsc_multi_string edge cases ---

    #[test]
    fn parse_pcsc_multi_string_empty() {
        let names = parse_pcsc_multi_string(&[]);
        assert!(names.is_empty());
    }

    #[test]
    fn parse_pcsc_multi_string_single_empty_terminator() {
        let names = parse_pcsc_multi_string(b"\0");
        assert!(names.is_empty());
    }

    #[test]
    fn parse_pcsc_multi_string_single_reader() {
        let names = parse_pcsc_multi_string(b"YubiKey\0\0");
        assert_eq!(names, vec!["YubiKey"]);
    }

    #[test]
    fn parse_pcsc_multi_string_no_trailing_null() {
        let names = parse_pcsc_multi_string(b"Reader1\0Reader2");
        assert_eq!(names, vec!["Reader1", "Reader2"]);
    }

    // --- build_select_apdu for Yubico OTP ---

    #[test]
    fn build_select_yubico_otp_apdu() {
        let apdu = build_select_apdu(&YUBICO_OTP_AID);
        assert_eq!(apdu[0..4], [0x00, 0xa4, 0x04, 0x00]);
        assert_eq!(apdu[4] as usize, YUBICO_OTP_AID.len());
        assert_eq!(&apdu[5..], &YUBICO_OTP_AID);
    }

    // --- FakeYubiKey behavior ---

    #[test]
    fn fake_key_info_has_expected_fields() {
        let key = FakeYubiKey::new(YubiKeySignatureAlgorithm::EcdsaP256Sha256);
        let info = key.key_info().unwrap();
        assert_eq!(info.slot, "9c");
        assert_eq!(info.algorithm, YubiKeySignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(info.public_key, vec![8, 6, 7, 5, 3, 0, 9]);
        assert_eq!(info.attestation_chain, vec![vec![1, 2, 3]]);
        assert_eq!(info.serial_number, Some("yk-serial-123".into()));
        assert_eq!(info.assurance_level, YubiKeyAssuranceLevel::PivAttested);
        assert_eq!(info.pin_policy, Some(YubiKeyPinPolicy::Always));
        assert_eq!(info.touch_policy, Some(YubiKeyTouchPolicy::Always));
        assert_eq!(info.generated_on_device, Some(true));
    }

    // --- checked_sign variants ---

    #[test]
    fn checked_sign_with_all_algorithm_types() {
        for alg in [
            YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256,
            YubiKeySignatureAlgorithm::RsaPssSha256,
            YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            YubiKeySignatureAlgorithm::EcdsaP384Sha384,
            YubiKeySignatureAlgorithm::Eddsa,
            YubiKeySignatureAlgorithm::Opaque("custom".into()),
        ] {
            let alg_name = format!("{:?}", alg);
            let key = FakeYubiKey::new(alg.clone());
            let sig = sign_record_with_yubikey_checked(&key, &[alg], "domain", &[1u8; 32]);
            assert!(sig.is_ok(), "failed for {}", alg_name);
        }
    }

    // --- signature_input_for_record ---

    #[test]
    fn signature_input_for_record_deterministic() {
        let a = signature_input_for_record("d", &[1, 2, 3]);
        let b = signature_input_for_record("d", &[1, 2, 3]);
        assert_eq!(a, b);
    }

    #[test]
    fn signature_input_for_record_different_inputs() {
        let a = signature_input_for_record("a", &[1u8; 32]);
        let b = signature_input_for_record("b", &[1u8; 32]);
        assert_ne!(a, b);
    }

    // --- YubiKeyPivInfo ---

    #[test]
    fn piv_info_clone_and_debug() {
        let info = YubiKeyPivInfo {
            piv_selected: true,
            yubico_otp_selected: false,
            version: Some([5, 4, 3]),
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
        let debug = format!("{:?}", info);
        assert!(debug.contains("piv_selected"));
    }

    // --- YubiKeyKeyInfo ---

    #[test]
    fn key_info_clone_and_equality() {
        let info = YubiKeyKeyInfo {
            slot: "9a".into(),
            algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0x04, 0x01, 0x02],
            attestation_chain: vec![vec![1, 2]],
            serial_number: Some("serial-1".into()),
            assurance_level: YubiKeyAssuranceLevel::HardwareBacked,
            pin_policy: Some(YubiKeyPinPolicy::Once),
            touch_policy: Some(YubiKeyTouchPolicy::Cached),
            generated_on_device: Some(false),
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn key_info_debug_contains_slot() {
        let info = YubiKeyKeyInfo {
            slot: "9e".into(),
            algorithm: YubiKeySignatureAlgorithm::Eddsa,
            public_key: vec![],
            attestation_chain: vec![],
            serial_number: None,
            assurance_level: YubiKeyAssuranceLevel::Unknown,
            pin_policy: None,
            touch_policy: None,
            generated_on_device: None,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("9e"));
    }

    // --- YubiKeyPivMetadata ---

    #[test]
    fn piv_metadata_clone_and_debug() {
        let meta = YubiKeyPivMetadata {
            slot: YubiKeyPivSlot::Retired(0x85),
            algorithm: Some(YubiKeySignatureAlgorithm::EcdsaP256Sha256),
            pin_policy: Some(YubiKeyPinPolicy::Default),
            touch_policy: None,
            generated_on_device: Some(true),
            public_key: vec![0xAA],
            raw_tlv: vec![0x53, 0x01, 0x11],
        };
        let cloned = meta.clone();
        assert_eq!(meta, cloned);
        let debug = format!("{:?}", meta);
        // 0x85 = 133 decimal
        assert!(debug.contains("Retired"));
        assert!(debug.contains("133"));
    }

    // --- YubiKeyReaderInfo ---

    #[test]
    fn reader_info_clone_and_debug() {
        let info = YubiKeyReaderInfo {
            name: "Test Reader".into(),
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
        assert_eq!(
            format!("{:?}", info),
            "YubiKeyReaderInfo { name: \"Test Reader\" }"
        );
    }

    // --- Looks-like-yubikey additional cases ---

    #[test]
    fn looks_like_yubikey_reader_case_insensitive() {
        assert!(looks_like_yubikey_reader("yubikey 5 nfc"));
        assert!(looks_like_yubikey_reader("YUBIKEY"));
        assert!(looks_like_yubikey_reader("Yubi"));
        assert!(!looks_like_yubikey_reader("OMNIKEY"));
        assert!(!looks_like_yubikey_reader("Gemalto"));
    }
}
