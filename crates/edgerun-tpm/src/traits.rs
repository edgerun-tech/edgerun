use alloc::vec::Vec;

use crate::types::{TpmError, TpmKeyInfo};

/// Abstract interface for a TPM-backed signing key.
pub trait TpmSigningKey {
    fn key_info(&self) -> Result<TpmKeyInfo, TpmError>;
    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError>;
}

/// Abstract byte-level TPM transport (e.g. `/dev/tpmrm0`, simulator, fake).
pub trait TpmTransport {
    fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError>;
}
