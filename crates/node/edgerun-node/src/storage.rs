//! Storage capability boundary for runtime-managed apps.
//!
//! The node authorizes storage capability use, but actual persistence is a
//! provider boundary. Runtime code depends on this trait instead of owning a
//! concrete filesystem or database.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::runtime::RuntimeError;

pub trait RuntimeStorage {
    fn read(&mut self, namespace: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn write(&mut self, namespace: &[u8], key: &[u8], value: &[u8]) -> Result<(), RuntimeError>;
}

#[cfg(any(feature = "std", test))]
#[derive(Default)]
pub struct MemoryRuntimeStorage {
    objects: BTreeMap<(Vec<u8>, [u8; 32]), Vec<u8>>,
}

#[cfg(any(feature = "std", test))]
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
