//! Storage capability boundary for runtime-managed apps.
//!
//! The node authorizes storage capability use, but actual persistence is a
//! provider boundary. Runtime code depends on this trait instead of owning a
//! concrete filesystem or database.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::runtime::RuntimeError;

/// Storage surface available to this node runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeStorageSurface {
    /// In-memory storage for tests, ephemeral wasm sessions, and replay.
    Memory,
    /// Native host durable storage adapter.
    HostDurable,
    /// Browser-provided durable storage adapter.
    BrowserDurable,
    /// No storage provider can realize this namespace.
    Unavailable,
}

/// A requested storage namespace.
///
/// This is provider intent, not permission to open files or browser storage.
/// The same runtime config can request the namespace; the deployment boundary
/// decides how that namespace is realized for native, wasm, replay, or tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStorageIntent {
    pub namespace: Vec<u8>,
    pub surface: RuntimeStorageSurface,
}

/// Runtime decision for a requested storage namespace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeStorageDecision {
    Provider(RuntimeStorageIntent),
    Denied(RuntimeStorageIntent),
}

pub trait RuntimeStorage {
    fn read(&mut self, namespace: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn write(&mut self, namespace: &[u8], key: &[u8], value: &[u8]) -> Result<(), RuntimeError>;
}

pub fn storage_intents(
    namespaces: &[Vec<u8>],
    surface: RuntimeStorageSurface,
) -> Vec<RuntimeStorageIntent> {
    namespaces
        .iter()
        .cloned()
        .map(|namespace| RuntimeStorageIntent { namespace, surface })
        .collect()
}

pub fn decide_storage(intent: RuntimeStorageIntent) -> RuntimeStorageDecision {
    match intent.surface {
        RuntimeStorageSurface::Memory
        | RuntimeStorageSurface::HostDurable
        | RuntimeStorageSurface::BrowserDurable => RuntimeStorageDecision::Provider(intent),
        RuntimeStorageSurface::Unavailable => RuntimeStorageDecision::Denied(intent),
    }
}

pub fn decide_storage_intents(intents: Vec<RuntimeStorageIntent>) -> Vec<RuntimeStorageDecision> {
    intents.into_iter().map(decide_storage).collect()
}

#[derive(Default)]
pub struct MemoryRuntimeStorage {
    objects: BTreeMap<(Vec<u8>, [u8; 32]), Vec<u8>>,
}

impl RuntimeStorage for MemoryRuntimeStorage {
    fn read(&mut self, namespace: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, RuntimeError> {
        let key: [u8; 32] = key.try_into().map_err(|_| RuntimeError::StorageDenied)?;
        Ok(self.objects.get(&(namespace.to_vec(), key)).cloned())
    }

    fn write(&mut self, namespace: &[u8], key: &[u8], value: &[u8]) -> Result<(), RuntimeError> {
        let key: [u8; 32] = key.try_into().map_err(|_| RuntimeError::StorageDenied)?;
        self.objects
            .insert((namespace.to_vec(), key), value.to_vec());
        Ok(())
    }
}
