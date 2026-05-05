use crate::prelude::v1::*;
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

/// Byte-level TPM transport that can write a response into caller-owned storage.
pub trait FixedTpmTransport {
    fn transact_into(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, TpmError>;
}
