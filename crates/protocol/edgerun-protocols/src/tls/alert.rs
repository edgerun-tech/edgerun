//! TLS alert protocol

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::fmt;

/// Alert level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    /// Warning (connection can continue)
    Warning = 1,
    /// Fatal (connection must be terminated)
    Fatal = 2,
}

impl AlertLevel {
    /// Parse from wire format
    pub fn from_wire(value: u8) -> Result<Self, String> {
        match value {
            1 => Ok(AlertLevel::Warning),
            2 => Ok(AlertLevel::Fatal),
            _ => Err(format!("Invalid alert level: {}", value)),
        }
    }
}

/// TLS alert description
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    /// close_notify
    CloseNotify = 0,
    /// handshake_failure
    HandshakeFailure = 40,
    /// bad_certificate
    BadCertificate = 42,
    /// unsupported_certificate
    UnsupportedCertificate = 43,
    /// certificate_revoked
    CertificateRevoked = 44,
    /// certificate_expired
    CertificateExpired = 45,
    /// certificate_unknown
    CertificateUnknown = 46,
    /// illegal_parameter
    IllegalParameter = 47,
    /// unknown_ca
    UnknownCA = 48,
    /// access_denied
    AccessDenied = 49,
    /// decode_error
    DecodeError = 50,
    /// decrypt_error
    DecryptError = 51,
    /// protocol_version
    ProtocolVersion = 70,
    /// insufficient_security
    InsufficientSecurity = 71,
    /// internal_error
    InternalError = 80,
    /// inappropriate_fallback
    InappropriateFallback = 86,
    /// user_canceled
    UserCanceled = 90,
    /// missing_extension
    MissingExtension = 109,
    /// unsupported_extension
    UnsupportedExtension = 110,
    /// unrecognized_name
    UnrecognizedName = 112,
    /// bad_certificate_status_response
    BadCertificateStatusResponse = 113,
    /// unknown_psk_identity
    UnknownPSKIdentity = 115,
    /// certificate_required
    CertificateRequired = 116,
    /// no_application_protocol
    NoApplicationProtocol = 120,
}

impl Alert {
    /// Parse from wire format
    pub fn from_wire(value: u8) -> Result<Self, String> {
        match value {
            0 => Ok(Alert::CloseNotify),
            40 => Ok(Alert::HandshakeFailure),
            42 => Ok(Alert::BadCertificate),
            43 => Ok(Alert::UnsupportedCertificate),
            44 => Ok(Alert::CertificateRevoked),
            45 => Ok(Alert::CertificateExpired),
            46 => Ok(Alert::CertificateUnknown),
            47 => Ok(Alert::IllegalParameter),
            48 => Ok(Alert::UnknownCA),
            49 => Ok(Alert::AccessDenied),
            50 => Ok(Alert::DecodeError),
            51 => Ok(Alert::DecryptError),
            70 => Ok(Alert::ProtocolVersion),
            71 => Ok(Alert::InsufficientSecurity),
            80 => Ok(Alert::InternalError),
            86 => Ok(Alert::InappropriateFallback),
            90 => Ok(Alert::UserCanceled),
            109 => Ok(Alert::MissingExtension),
            110 => Ok(Alert::UnsupportedExtension),
            112 => Ok(Alert::UnrecognizedName),
            113 => Ok(Alert::BadCertificateStatusResponse),
            115 => Ok(Alert::UnknownPSKIdentity),
            116 => Ok(Alert::CertificateRequired),
            120 => Ok(Alert::NoApplicationProtocol),
            _ => Err(format!("Unknown alert: {}", value)),
        }
    }

    /// Get alert description as string
    pub fn description(&self) -> &'static str {
        match self {
            Alert::CloseNotify => "close_notify",
            Alert::HandshakeFailure => "handshake_failure",
            Alert::BadCertificate => "bad_certificate",
            Alert::UnsupportedCertificate => "unsupported_certificate",
            Alert::CertificateRevoked => "certificate_revoked",
            Alert::CertificateExpired => "certificate_expired",
            Alert::CertificateUnknown => "certificate_unknown",
            Alert::IllegalParameter => "illegal_parameter",
            Alert::UnknownCA => "unknown_ca",
            Alert::AccessDenied => "access_denied",
            Alert::DecodeError => "decode_error",
            Alert::DecryptError => "decrypt_error",
            Alert::ProtocolVersion => "protocol_version",
            Alert::InsufficientSecurity => "insufficient_security",
            Alert::InternalError => "internal_error",
            Alert::InappropriateFallback => "inappropriate_fallback",
            Alert::UserCanceled => "user_canceled",
            Alert::MissingExtension => "missing_extension",
            Alert::UnsupportedExtension => "unsupported_extension",
            Alert::UnrecognizedName => "unrecognized_name",
            Alert::BadCertificateStatusResponse => "bad_certificate_status_response",
            Alert::UnknownPSKIdentity => "unknown_psk_identity",
            Alert::CertificateRequired => "certificate_required",
            Alert::NoApplicationProtocol => "no_application_protocol",
        }
    }
}

impl fmt::Display for Alert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Alert message
#[derive(Debug, Clone)]
pub struct AlertMessage {
    /// Alert level
    pub level: AlertLevel,
    /// Alert description
    pub description: Alert,
}

impl AlertMessage {
    /// Create a new alert message
    pub fn new(level: AlertLevel, description: Alert) -> Self {
        AlertMessage { level, description }
    }

    /// Parse alert from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 2 {
            return Err("Alert too short".to_string());
        }

        let level = AlertLevel::from_wire(data[0])?;
        let description = Alert::from_wire(data[1])?;

        Ok(AlertMessage { level, description })
    }

    /// Serialize alert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        vec![self.level as u8, self.description as u8]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_level_wire() {
        assert_eq!(AlertLevel::from_wire(1).unwrap(), AlertLevel::Warning);
        assert_eq!(AlertLevel::from_wire(2).unwrap(), AlertLevel::Fatal);
        assert!(AlertLevel::from_wire(3).is_err());
    }

    #[test]
    fn test_alert_from_wire() {
        assert_eq!(Alert::from_wire(40).unwrap(), Alert::HandshakeFailure);
        assert_eq!(Alert::from_wire(80).unwrap(), Alert::InternalError);
        assert!(Alert::from_wire(255).is_err());
    }

    #[test]
    fn test_alert_description() {
        assert_eq!(Alert::HandshakeFailure.description(), "handshake_failure");
        assert_eq!(Alert::InternalError.description(), "internal_error");
    }

    #[test]
    fn test_alert_display() {
        assert_eq!(Alert::UnrecognizedName.to_string(), "unrecognized_name");
    }

    #[test]
    fn test_alert_message_roundtrip() {
        let alert = AlertMessage::new(AlertLevel::Fatal, Alert::HandshakeFailure);
        let bytes = alert.to_bytes();
        let parsed = AlertMessage::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.level, alert.level);
        assert_eq!(parsed.description, alert.description);
    }

    #[test]
    fn test_alert_message_invalid() {
        assert!(AlertMessage::from_bytes(&[]).is_err());
        assert!(AlertMessage::from_bytes(&[1]).is_err());
    }
}
