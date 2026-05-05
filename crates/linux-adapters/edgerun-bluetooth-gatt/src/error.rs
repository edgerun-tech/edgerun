use crate::prelude::v1::*;
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GattError {
    InvalidParameter(String),
    ConnectionFailed(String),
    ConnectionRefused(String),
    HostUnreachable,
    AuthenticationFailed(String),
    NotConnected,
    AlreadyConnected,
    Timeout(u32),
    AttError(u8),
    AttErrorResponse { handle: u16, error: u8 },
    NotPermitted(String),
    ReadNotAllowed,
    WriteNotAllowed,
    InvalidOffset { offset: u16, max: u16 },
    InvalidLength { length: usize, max: usize },
    MtuExchangeFailed,
    MtuTooSmall { mtu: u16, min: u16 },
    ServiceNotFound(String),
    CharacteristicNotFound(String),
    DescriptorNotFound(String),
    NotificationNotEnabled,
    DeviceNotFound(String),
    DeviceBusy,
    InvalidUuid(String),
    InvalidAddress(String),
    SocketFailed(String),
    PollFailed(String),
    SendFailed(String),
    RecvFailed(String),
    ParseError(String),
    EncryptionError(String),
    DecryptionError(String),
    SessionNotEstablished,
    KeyExchangeFailed,
    CrcMismatch,
    PacketTooShort { actual: usize, min: usize },
    PacketTooLong { actual: usize, max: usize },
    IoError(String),
    PermissionDenied,
    Unknown(String),
}

impl fmt::Display for GattError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter(s) => write!(f, "invalid parameter: {}", s),
            Self::ConnectionFailed(s) => write!(f, "connection failed: {}", s),
            Self::ConnectionRefused(s) => write!(f, "connection refused: {}", s),
            Self::HostUnreachable => write!(f, "host unreachable"),
            Self::AuthenticationFailed(s) => write!(f, "authentication failed: {}", s),
            Self::NotConnected => write!(f, "not connected"),
            Self::AlreadyConnected => write!(f, "already connected"),
            Self::Timeout(ms) => write!(f, "timeout after {}ms", ms),
            Self::AttError(code) => write!(f, "ATT protocol error: {}", code),
            Self::AttErrorResponse { handle, error } => {
                write!(
                    f,
                    "ATT error response: handle=0x{:04x}, error={}",
                    handle, error
                )
            }
            Self::NotPermitted(s) => write!(f, "GATT operation not permitted: {}", s),
            Self::ReadNotAllowed => write!(f, "read not allowed"),
            Self::WriteNotAllowed => write!(f, "write not allowed"),
            Self::InvalidOffset { offset, max } => {
                write!(f, "invalid offset: {} (max={})", offset, max)
            }
            Self::InvalidLength { length, max } => {
                write!(f, "invalid length: {} (expected <= {})", length, max)
            }
            Self::MtuExchangeFailed => write!(f, "MTU exchange failed"),
            Self::MtuTooSmall { mtu, min } => {
                write!(f, "MTU too small: {} (minimum required: {})", mtu, min)
            }
            Self::ServiceNotFound(s) => write!(f, "service not found: {}", s),
            Self::CharacteristicNotFound(s) => write!(f, "characteristic not found: {}", s),
            Self::DescriptorNotFound(s) => write!(f, "descriptor not found: {}", s),
            Self::NotificationNotEnabled => write!(f, "notification not enabled"),
            Self::DeviceNotFound(s) => write!(f, "device not found: {}", s),
            Self::DeviceBusy => write!(f, "device busy"),
            Self::InvalidUuid(s) => write!(f, "invalid UUID: {}", s),
            Self::InvalidAddress(s) => write!(f, "invalid address format: {}", s),
            Self::SocketFailed(s) => write!(f, "socket operation failed: {}", s),
            Self::PollFailed(s) => write!(f, "poll failed: {}", s),
            Self::SendFailed(s) => write!(f, "send failed: {}", s),
            Self::RecvFailed(s) => write!(f, "recv failed: {}", s),
            Self::ParseError(s) => write!(f, "parse error: {}", s),
            Self::EncryptionError(s) => write!(f, "encryption error: {}", s),
            Self::DecryptionError(s) => write!(f, "decryption error: {}", s),
            Self::SessionNotEstablished => write!(f, "session not established"),
            Self::KeyExchangeFailed => write!(f, "key exchange failed"),
            Self::CrcMismatch => write!(f, "CRC mismatch"),
            Self::PacketTooShort { actual, min } => {
                write!(f, "packet too short: {} bytes (minimum {})", actual, min)
            }
            Self::PacketTooLong { actual, max } => {
                write!(f, "packet too long: {} bytes (maximum {})", actual, max)
            }
            Self::IoError(s) => write!(f, "IO error: {}", s),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::Unknown(s) => write!(f, "unknown error: {}", s),
        }
    }
}

impl core::error::Error for GattError {}

impl GattError {
    pub fn att_error_code(code: u8) -> Self {
        match code {
            0x01 => Self::InvalidOffset { offset: 0, max: 0 },
            0x02 => Self::InvalidLength { length: 0, max: 0 },
            0x03 => Self::NotPermitted("ATT operation not permitted".to_string()),
            0x04 => Self::ReadNotAllowed,
            0x05 => Self::WriteNotAllowed,
            0x06 => Self::AuthenticationFailed("authentication not sufficient".to_string()),
            0x07 => Self::AttError(0x07),
            0x08 => Self::AttError(0x08),
            0x09 => Self::Timeout(0),
            0x0a => Self::NotConnected,
            _ => Self::AttError(code),
        }
    }

    pub fn is_att_error(&self) -> bool {
        matches!(self, Self::AttError(_) | Self::AttErrorResponse { .. })
    }

    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            Self::ConnectionFailed(_)
                | Self::ConnectionRefused(_)
                | Self::HostUnreachable
                | Self::Timeout(_)
        )
    }

    pub fn is_authentication_error(&self) -> bool {
        matches!(
            self,
            Self::AuthenticationFailed(_) | Self::ReadNotAllowed | Self::WriteNotAllowed
        )
    }
}

pub type GattResult<T> = Result<T, GattError>;

impl From<std::io::Error> for GattError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::ConnectionRefused => Self::ConnectionRefused(err.to_string()),
            std::io::ErrorKind::ConnectionReset => Self::ConnectionFailed(err.to_string()),
            std::io::ErrorKind::HostUnreachable => Self::HostUnreachable,
            std::io::ErrorKind::TimedOut => Self::Timeout(0),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => Self::IoError(err.to_string()),
        }
    }
}

impl From<GattError> for edgerun_capabilities::CapabilityError {
    fn from(err: GattError) -> Self {
        edgerun_capabilities::CapabilityError::Provider(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn att_error_codes() {
        let err = GattError::att_error_code(0x01);
        assert!(matches!(err, GattError::InvalidOffset { .. }));

        let err2 = GattError::att_error_code(0x0a);
        assert!(matches!(err2, GattError::NotConnected));
    }

    #[test]
    fn error_classification() {
        assert!(GattError::ConnectionFailed("test".to_string()).is_connection_error());
        assert!(GattError::HostUnreachable.is_connection_error());

        assert!(!GattError::att_error_code(0x01).is_authentication_error());

        assert!(GattError::ReadNotAllowed.is_authentication_error());
    }

    #[test]
    fn io_error_conversion() {
        use std::io;

        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "test");
        let gatt_err = GattError::from(io_err);
        assert!(matches!(gatt_err, GattError::PermissionDenied));
    }
}
