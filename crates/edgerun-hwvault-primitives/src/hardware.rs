// SPDX-License-Identifier: Apache-2.0
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_TPM_NV_INDEX: u32 = 0x0150_0016;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareSecurityMode {
    TpmRequired,
    AllowSoftwareFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareBackend {
    Tpm,
    Software,
}

#[derive(Debug, Clone)]
pub struct DeviceSigner {
    signing: SigningKey,
    pub public_key_b64url: String,
    pub backend: HardwareBackend,
}

impl DeviceSigner {
    pub fn sign_b64url(&self, message: &[u8]) -> String {
        let sig = self.signing.sign(message);
        URL_SAFE_NO_PAD.encode(sig.to_bytes())
    }

    pub fn build_handshake(
        &self,
        owner_pubkey: &str,
        nonce_b64url: &str,
        issued_at_unix_s: u64,
    ) -> Result<DeviceHandshake, HardwareError> {
        if owner_pubkey.trim().is_empty() {
            return Err(HardwareError::InvalidIdentity);
        }
        if nonce_b64url.trim().is_empty() || nonce_b64url.len() > 256 {
            return Err(HardwareError::InvalidIdentity);
        }
        let payload = DeviceHandshakePayload {
            version: 1,
            owner_pubkey: owner_pubkey.trim().to_string(),
            device_pubkey_b64url: self.public_key_b64url.clone(),
            nonce_b64url: nonce_b64url.trim().to_string(),
            issued_at_unix_s,
        };
        let canonical = canonical_handshake_message(&payload);
        let signature_b64url = self.sign_b64url(canonical.as_bytes());
        Ok(DeviceHandshake {
            payload,
            canonical,
            signature_b64url,
            backend: self.backend,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHandshakePayload {
    pub version: u8,
    pub owner_pubkey: String,
    pub device_pubkey_b64url: String,
    pub nonce_b64url: String,
    pub issued_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHandshake {
    pub payload: DeviceHandshakePayload,
    pub canonical: String,
    pub signature_b64url: String,
    pub backend: HardwareBackend,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeviceIdentityRecord {
    version: u8,
    backend: HardwareBackend,
    public_key_b64url: String,
    tpm_nv_index: Option<u32>,
    software_seed_b64url: Option<String>,
}

#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("failed to resolve home directory")]
    NoHomeDir,
    #[error("hardware keystore file IO failed: {0}")]
    Io(String),
    #[error("hardware keystore record parse failed: {0}")]
    Parse(String),
    #[error("TPM is required but not available")]
    TpmRequiredUnavailable,
    #[error("TPM command failed: {0}")]
    TpmCommand(String),
    #[error("stored identity is invalid")]
    InvalidIdentity,
}

pub fn load_or_create_device_signer(mode: HardwareSecurityMode) -> Result<DeviceSigner, HardwareError> {
    let record_path = default_record_path()?;
    if let Some(existing) = read_record(&record_path)? {
        if let Ok(signer) = signer_from_record(&existing, mode) {
            return Ok(signer);
        }
    }

    if tpm_available() {
        if let Ok(signer) = create_tpm_signer(&record_path, DEFAULT_TPM_NV_INDEX) {
            return Ok(signer);
        }
    }

    if mode == HardwareSecurityMode::AllowSoftwareFallback {
        return create_software_signer(&record_path);
    }

    Err(HardwareError::TpmRequiredUnavailable)
}

pub fn random_token_b64url(num_bytes: usize) -> String {
    let mut bytes = vec![0_u8; num_bytes];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn signer_from_record(
    record: &DeviceIdentityRecord,
    mode: HardwareSecurityMode,
) -> Result<DeviceSigner, HardwareError> {
    match record.backend {
        HardwareBackend::Tpm => {
            if !tpm_available() {
                return Err(HardwareError::TpmRequiredUnavailable);
            }
            let index = record.tpm_nv_index.unwrap_or(DEFAULT_TPM_NV_INDEX);
            let seed = tpm_unseal_seed(index)?;
            let signing = SigningKey::from_bytes(&seed);
            let public_key_b64url = URL_SAFE_NO_PAD.encode(signing.verifying_key().to_bytes());
            if public_key_b64url != record.public_key_b64url {
                return Err(HardwareError::InvalidIdentity);
            }
            Ok(DeviceSigner {
                signing,
                public_key_b64url,
                backend: HardwareBackend::Tpm,
            })
        }
        HardwareBackend::Software => {
            if mode == HardwareSecurityMode::TpmRequired {
                return Err(HardwareError::TpmRequiredUnavailable);
            }
            let Some(seed_b64) = record.software_seed_b64url.as_deref() else {
                return Err(HardwareError::InvalidIdentity);
            };
            let seed = decode_seed(seed_b64)?;
            let signing = SigningKey::from_bytes(&seed);
            let public_key_b64url = URL_SAFE_NO_PAD.encode(signing.verifying_key().to_bytes());
            if public_key_b64url != record.public_key_b64url {
                return Err(HardwareError::InvalidIdentity);
            }
            Ok(DeviceSigner {
                signing,
                public_key_b64url,
                backend: HardwareBackend::Software,
            })
        }
    }
}

fn create_tpm_signer(record_path: &Path, nv_index: u32) -> Result<DeviceSigner, HardwareError> {
    let mut seed = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    tpm_seal_seed(nv_index, &seed)?;

    let signing = SigningKey::from_bytes(&seed);
    let public_key_b64url = URL_SAFE_NO_PAD.encode(signing.verifying_key().to_bytes());
    let record = DeviceIdentityRecord {
        version: 1,
        backend: HardwareBackend::Tpm,
        public_key_b64url: public_key_b64url.clone(),
        tpm_nv_index: Some(nv_index),
        software_seed_b64url: None,
    };
    write_record(record_path, &record)?;

    Ok(DeviceSigner {
        signing,
        public_key_b64url,
        backend: HardwareBackend::Tpm,
    })
}

fn create_software_signer(record_path: &Path) -> Result<DeviceSigner, HardwareError> {
    let mut seed = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing = SigningKey::from_bytes(&seed);
    let public_key_b64url = URL_SAFE_NO_PAD.encode(signing.verifying_key().to_bytes());
    let record = DeviceIdentityRecord {
        version: 1,
        backend: HardwareBackend::Software,
        public_key_b64url: public_key_b64url.clone(),
        tpm_nv_index: None,
        software_seed_b64url: Some(URL_SAFE_NO_PAD.encode(seed)),
    };
    write_record(record_path, &record)?;

    Ok(DeviceSigner {
        signing,
        public_key_b64url,
        backend: HardwareBackend::Software,
    })
}

fn decode_seed(seed_b64: &str) -> Result<[u8; 32], HardwareError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(seed_b64.as_bytes())
        .map_err(|e| HardwareError::Parse(format!("invalid seed b64: {e}")))?;
    decoded
        .as_slice()
        .try_into()
        .map_err(|_| HardwareError::Parse("seed must decode to 32 bytes".to_string()))
}

fn read_record(path: &Path) -> Result<Option<DeviceIdentityRecord>, HardwareError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|e| HardwareError::Io(e.to_string()))?;
    let record = serde_json::from_str::<DeviceIdentityRecord>(&raw)
        .map_err(|e| HardwareError::Parse(e.to_string()))?;
    Ok(Some(record))
}

fn write_record(path: &Path, record: &DeviceIdentityRecord) -> Result<(), HardwareError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| HardwareError::Io(e.to_string()))?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|e| HardwareError::Io(e.to_string()))?;
    }
    let payload = serde_json::to_string_pretty(record)
        .map_err(|e| HardwareError::Parse(e.to_string()))?;
    fs::write(path, payload).map_err(|e| HardwareError::Io(e.to_string()))
}

fn default_record_path() -> Result<PathBuf, HardwareError> {
    let home = std::env::var("HOME").map_err(|_| HardwareError::NoHomeDir)?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("edgerun")
        .join("device_identity.json"))
}

fn canonical_handshake_message(payload: &DeviceHandshakePayload) -> String {
    format!(
        "edgerun-device-handshake-v{}|{}|{}|{}|{}",
        payload.version,
        payload.owner_pubkey,
        payload.device_pubkey_b64url,
        payload.nonce_b64url,
        payload.issued_at_unix_s
    )
}

fn tpm_available() -> bool {
    (Path::new("/dev/tpmrm0").exists() || Path::new("/dev/tpm0").exists())
        && command_exists("tpm2_nvdefine")
        && command_exists("tpm2_nvwrite")
        && command_exists("tpm2_nvread")
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn tpm_seal_seed(nv_index: u32, seed: &[u8; 32]) -> Result<(), HardwareError> {
    let index_arg = format!("0x{nv_index:08x}");

    let exists = Command::new("tpm2_nvreadpublic")
        .arg(&index_arg)
        .output()
        .map_err(|e| HardwareError::TpmCommand(format!("nvreadpublic spawn failed: {e}")))?;

    if !exists.status.success() {
        let define = Command::new("tpm2_nvdefine")
            .args([&index_arg, "-C", "o", "-s", "32", "-a", "ownerread|ownerwrite"])
            .output()
            .map_err(|e| HardwareError::TpmCommand(format!("nvdefine spawn failed: {e}")))?;
        if !define.status.success() {
            return Err(HardwareError::TpmCommand(format!(
                "nvdefine failed: {}",
                String::from_utf8_lossy(&define.stderr)
            )));
        }
    }

    let tmp_path = std::env::temp_dir().join(format!("edgerun-seed-{nv_index}.bin"));
    fs::write(&tmp_path, seed).map_err(|e| HardwareError::Io(e.to_string()))?;
    let write = Command::new("tpm2_nvwrite")
        .args([&index_arg, "-C", "o", "-i"])
        .arg(&tmp_path)
        .output()
        .map_err(|e| HardwareError::TpmCommand(format!("nvwrite spawn failed: {e}")))?;
    let _ = fs::remove_file(&tmp_path);
    if !write.status.success() {
        return Err(HardwareError::TpmCommand(format!(
            "nvwrite failed: {}",
            String::from_utf8_lossy(&write.stderr)
        )));
    }

    Ok(())
}

fn tpm_unseal_seed(nv_index: u32) -> Result<[u8; 32], HardwareError> {
    let index_arg = format!("0x{nv_index:08x}");
    let tmp_path = std::env::temp_dir().join(format!("edgerun-unseal-{nv_index}.bin"));

    let read = Command::new("tpm2_nvread")
        .args([&index_arg, "-C", "o", "-s", "32", "-o"])
        .arg(&tmp_path)
        .output()
        .map_err(|e| HardwareError::TpmCommand(format!("nvread spawn failed: {e}")))?;

    if !read.status.success() {
        let _ = fs::remove_file(&tmp_path);
        return Err(HardwareError::TpmCommand(format!(
            "nvread failed: {}",
            String::from_utf8_lossy(&read.stderr)
        )));
    }

    let data = fs::read(&tmp_path).map_err(|e| HardwareError::Io(e.to_string()))?;
    let _ = fs::remove_file(&tmp_path);

    data.as_slice()
        .try_into()
        .map_err(|_| HardwareError::Parse("TPM seed must be 32 bytes".to_string()))
}
