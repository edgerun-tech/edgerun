use crate::prelude::v1::*;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use alloc::vec;
use alloc::vec::Vec;

use crate::device::TpmDevice;
use crate::signing::{sign_params_for_message, sign_prehashed_with_device};
use crate::traits::{FixedTpmTransport, TpmSigningKey, TpmTransport};
use crate::types::*;
use crate::wire::encode_parsed_signature;

/// Concrete TPM transport via `/dev/tpmrm0` on Linux.
#[derive(Clone, Debug)]
pub struct LinuxTpmDevice {
    path: PathBuf,
}

impl LinuxTpmDevice {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TpmTransport for LinuxTpmDevice {
    fn transact(&mut self, command: &[u8]) -> Result<Vec<u8>, TpmError> {
        use crate::wire::parse::parse_response_header;

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| TpmError::Io(format!("open {}: {err}", self.path.display())))?;
        file.write_all(command)
            .map_err(|err| TpmError::Io(format!("write {}: {err}", self.path.display())))?;

        let mut header_buf = [0u8; 10];
        file.read_exact(&mut header_buf)
            .map_err(|err| TpmError::Io(format!("read header {}: {err}", self.path.display())))?;
        let header = parse_response_header(&header_buf)?;
        if header.size < header_buf.len() as u32 {
            return Err(TpmError::Protocol(format!(
                "response too small: {}",
                header.size
            )));
        }
        let mut out = Vec::with_capacity(header.size as usize);
        out.extend_from_slice(&header_buf);
        let remaining = header.size as usize - header_buf.len();
        if remaining > 0 {
            let mut rest = vec![0u8; remaining];
            file.read_exact(&mut rest)
                .map_err(|err| TpmError::Io(format!("read body {}: {err}", self.path.display())))?;
            out.extend_from_slice(&rest);
        }
        Ok(out)
    }
}

impl FixedTpmTransport for LinuxTpmDevice {
    fn transact_into(&mut self, command: &[u8], response: &mut [u8]) -> Result<usize, TpmError> {
        let owned = self.transact(command)?;
        if owned.len() > response.len() {
            return Err(TpmError::Protocol(
                "TPM response exceeds fixed buffer".into(),
            ));
        }
        response[..owned.len()].copy_from_slice(&owned);
        Ok(owned.len())
    }
}

/// Concrete `TpmSigningKey` backed by a Linux TPM device (`/dev/tpmrm0`).
#[derive(Clone, Debug)]
pub struct LinuxTpmSigningKey {
    device_path: PathBuf,
    handle: TpmHandle,
    authorization_mode: TpmAuthorizationMode,
}

impl LinuxTpmSigningKey {
    pub fn new(device_path: impl Into<PathBuf>, handle: TpmHandle) -> Self {
        Self {
            device_path: device_path.into(),
            handle,
            authorization_mode: TpmAuthorizationMode::None,
        }
    }

    pub fn with_auth_value(mut self, auth_value: impl Into<Vec<u8>>) -> Self {
        self.authorization_mode = TpmAuthorizationMode::AuthValue(TpmAuthValueSession {
            auth_value: auth_value.into(),
            session_attributes: 0,
        });
        self
    }

    pub fn with_auth_value_session(mut self, auth: TpmAuthValueSession) -> Self {
        self.authorization_mode = TpmAuthorizationMode::AuthValue(auth);
        self
    }

    pub fn with_policy_session_runner(mut self, runner: TpmPolicySessionRunner) -> Self {
        self.authorization_mode = TpmAuthorizationMode::Policy(runner);
        self
    }

    pub fn device_path(&self) -> &Path {
        &self.device_path
    }

    pub fn handle(&self) -> TpmHandle {
        self.handle
    }

    pub fn authorization_mode(&self) -> &TpmAuthorizationMode {
        &self.authorization_mode
    }
}

impl TpmSigningKey for LinuxTpmSigningKey {
    fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
        let mut device = TpmDevice::new(LinuxTpmDevice::new(self.device_path.clone()));
        device.read_key_info(self.handle)
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
        let key_info = self.key_info()?;
        let params = sign_params_for_message(self.handle, &key_info.algorithm, message)?;
        let mut device = TpmDevice::new(LinuxTpmDevice::new(self.device_path.clone()));
        let parsed = sign_prehashed_with_device(&mut device, &self.authorization_mode, &params)?;
        Ok(encode_parsed_signature(&parsed))
    }
}
