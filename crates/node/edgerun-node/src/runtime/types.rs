use alloc::string::String;
use alloc::vec::Vec;

use edgerun_protocols::wire::{RuntimeAppMessage, RuntimeEvent, RuntimeRoutedAppMessage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidWireRecord,
    AppNotInstalled,
    RouteDenied,
    RouteNotFound,
    IdentityRouteNotFound,
    SigningDenied,
    SigningProvider(String),
    StorageDenied,
    StorageProvider(String),
}

impl core::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidWireRecord => f.write_str("invalid runtime wire record"),
            Self::AppNotInstalled => f.write_str("app is not installed"),
            Self::RouteDenied => f.write_str("route is not declared by installed app"),
            Self::RouteNotFound => f.write_str("no route matched request"),
            Self::IdentityRouteNotFound => f.write_str("no identity route matched recipient"),
            Self::SigningDenied => f.write_str("signing capability denied"),
            Self::SigningProvider(error) => write!(f, "signing provider failed: {error}"),
            Self::StorageDenied => f.write_str("storage namespace is not granted to app"),
            Self::StorageProvider(error) => write!(f, "storage provider failed: {error}"),
        }
    }
}

impl core::error::Error for RuntimeError {}

pub trait RuntimeSigner {
    fn runtime_id(&self) -> [u8; 32];
    fn sign_event_payload(&mut self, payload_sha256: &[u8; 32]) -> Vec<u8>;
}

pub trait RuntimeAppSigner {
    fn app_public_key(&mut self, app_id: &[u8; 32]) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn sign_app_payload(
        &mut self,
        app_id: &[u8; 32],
        payload: &[u8],
    ) -> Result<Vec<u8>, RuntimeError>;
}

pub struct UnsignedRuntimeSigner {
    runtime_id: [u8; 32],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopRuntimeAppSigner;

impl UnsignedRuntimeSigner {
    pub const fn new(runtime_id: [u8; 32]) -> Self {
        Self { runtime_id }
    }
}

impl RuntimeSigner for UnsignedRuntimeSigner {
    fn runtime_id(&self) -> [u8; 32] {
        self.runtime_id
    }

    fn sign_event_payload(&mut self, _payload_sha256: &[u8; 32]) -> Vec<u8> {
        Vec::new()
    }
}

impl RuntimeAppSigner for NoopRuntimeAppSigner {
    fn app_public_key(&mut self, _app_id: &[u8; 32]) -> Result<Option<Vec<u8>>, RuntimeError> {
        Ok(None)
    }

    fn sign_app_payload(
        &mut self,
        _app_id: &[u8; 32],
        _payload: &[u8],
    ) -> Result<Vec<u8>, RuntimeError> {
        Err(RuntimeError::SigningDenied)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLogEntry {
    pub event: RuntimeEvent,
    pub event_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeMessageDelivery {
    Local(RuntimeAppMessage),
    Remote(RuntimeRoutedAppMessage),
}
