// SPDX-License-Identifier: GPL-2.0-only
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::path::Path;
use thiserror::Error;

const WRAPPED_KEY_MAGIC: &[u8; 4] = b"EKW1";
const WRAPPED_NONCE_LEN: usize = 24;
const WRAPPED_CT_LEN: usize = 32 + 16; // 32-byte key + Poly1305 tag

#[derive(Error, Debug)]
pub enum KeyManagementError {
    #[error("invalid key format: {0}")]
    InvalidFormat(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("crypto error")]
    Crypto,
    #[error("provider unavailable: {0}")]
    Unavailable(String),
}

pub type KeyResult<T> = Result<T, KeyManagementError>;

pub trait KeyProvider: Send + Sync {
    fn load_store_key(&self, store_uuid: [u8; 16]) -> KeyResult<[u8; 32]>;
}

#[derive(Debug, Clone)]
pub struct EnvKeyProvider {
    pub var_name: String,
}

impl EnvKeyProvider {
    pub fn new(var_name: impl Into<String>) -> Self {
        Self {
            var_name: var_name.into(),
        }
    }
}

impl KeyProvider for EnvKeyProvider {
    fn load_store_key(&self, _store_uuid: [u8; 16]) -> KeyResult<[u8; 32]> {
        let v = std::env::var(&self.var_name).map_err(|_| {
            KeyManagementError::Unavailable(format!("env var {} not set", self.var_name))
        })?;
        parse_hex_key(&v)
    }
}

#[derive(Debug, Clone)]
pub struct PassphraseKeyProvider {
    passphrase: String,
    salt_prefix: Vec<u8>,
    params: Params,
}

impl PassphraseKeyProvider {
    pub fn new(passphrase: impl Into<String>) -> Self {
        Self {
            passphrase: passphrase.into(),
            salt_prefix: b"erfs/argon2id/".to_vec(),
            params: Params::new(64 * 1024, 3, 1, Some(32)).expect("valid argon2 params"),
        }
    }
}

impl KeyProvider for PassphraseKeyProvider {
    fn load_store_key(&self, store_uuid: [u8; 16]) -> KeyResult<[u8; 32]> {
        let mut salt = self.salt_prefix.clone();
        salt.extend_from_slice(&store_uuid);
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone());
        let mut out = [0u8; 32];
        argon2
            .hash_password_into(self.passphrase.as_bytes(), &salt, &mut out)
            .map_err(|_| KeyManagementError::Crypto)?;
        Ok(out)
    }
}

#[derive(Debug, Clone)]
pub struct WrappedFileKeyProvider {
    path: std::path::PathBuf,
    wrapping_key: [u8; 32],
}

impl WrappedFileKeyProvider {
    pub fn new(path: impl Into<std::path::PathBuf>, wrapping_key: [u8; 32]) -> Self {
        Self {
            path: path.into(),
            wrapping_key,
        }
    }

    pub fn write_wrapped_key(
        path: impl AsRef<Path>,
        store_key: [u8; 32],
        wrapping_key: [u8; 32],
    ) -> KeyResult<()> {
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&wrapping_key));
        let mut nonce = [0u8; WRAPPED_NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let ct = cipher
            .encrypt(XNonce::from_slice(&nonce), store_key.as_slice())
            .map_err(|_| KeyManagementError::Crypto)?;
        if ct.len() != WRAPPED_CT_LEN {
            return Err(KeyManagementError::InvalidFormat(
                "wrapped key ciphertext length mismatch".to_string(),
            ));
        }
        let mut out = Vec::with_capacity(4 + WRAPPED_NONCE_LEN + WRAPPED_CT_LEN);
        out.extend_from_slice(WRAPPED_KEY_MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        std::fs::write(path, out)?;
        Ok(())
    }
}

impl KeyProvider for WrappedFileKeyProvider {
    fn load_store_key(&self, _store_uuid: [u8; 16]) -> KeyResult<[u8; 32]> {
        let blob = std::fs::read(&self.path)?;
        if blob.len() != 4 + WRAPPED_NONCE_LEN + WRAPPED_CT_LEN {
            return Err(KeyManagementError::InvalidFormat(
                "invalid wrapped key blob length".to_string(),
            ));
        }
        if &blob[0..4] != WRAPPED_KEY_MAGIC {
            return Err(KeyManagementError::InvalidFormat(
                "invalid wrapped key magic".to_string(),
            ));
        }
        let mut nonce = [0u8; WRAPPED_NONCE_LEN];
        nonce.copy_from_slice(&blob[4..(4 + WRAPPED_NONCE_LEN)]);
        let ct = &blob[(4 + WRAPPED_NONCE_LEN)..];
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.wrapping_key));
        let plain = cipher
            .decrypt(XNonce::from_slice(&nonce), ct)
            .map_err(|_| KeyManagementError::Crypto)?;
        if plain.len() != 32 {
            return Err(KeyManagementError::InvalidFormat(
                "invalid unwrapped key length".to_string(),
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&plain);
        Ok(key)
    }
}

#[derive(Debug, Clone)]
pub struct LinuxKeyringProvider {
    pub key_name: String,
}

impl LinuxKeyringProvider {
    pub fn new(key_name: impl Into<String>) -> Self {
        Self {
            key_name: key_name.into(),
        }
    }
}

impl KeyProvider for LinuxKeyringProvider {
    fn load_store_key(&self, _store_uuid: [u8; 16]) -> KeyResult<[u8; 32]> {
        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("keyctl")
                .args(["pipe", &self.key_name])
                .output();
            let output = match output {
                Ok(o) => o,
                Err(e) => {
                    return Err(KeyManagementError::Unavailable(format!(
                        "keyctl not available: {e}"
                    )))
                }
            };
            if !output.status.success() {
                return Err(KeyManagementError::Unavailable(
                    "failed reading key from keyctl".to_string(),
                ));
            }
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return parse_hex_key(&text);
        }
        #[allow(unreachable_code)]
        Err(KeyManagementError::Unavailable(
            "linux keyring provider requires linux target".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct AndroidKeystoreProvider;

impl KeyProvider for AndroidKeystoreProvider {
    fn load_store_key(&self, _store_uuid: [u8; 16]) -> KeyResult<[u8; 32]> {
        Err(KeyManagementError::Unavailable(
            "android keystore adapter is not wired yet; use wrapped-file backend with keystore-derived wrapping key".to_string(),
        ))
    }
}

pub fn derive_wrapping_key_from_material(material: &[u8], context: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(context), material);
    let mut out = [0u8; 32];
    hk.expand(b"erfs/wrapping-key", &mut out)
        .expect("fixed-size HKDF expand cannot fail");
    out
}

fn parse_hex_key(v: &str) -> KeyResult<[u8; 32]> {
    let hex = v.trim();
    if hex.len() != 64 {
        return Err(KeyManagementError::InvalidFormat(
            "expected 64 hex chars for 32-byte key".to_string(),
        ));
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        let hi = hex_to_nibble(hex.as_bytes()[2 * i])?;
        let lo = hex_to_nibble(hex.as_bytes()[2 * i + 1])?;
        *b = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_to_nibble(b: u8) -> KeyResult<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(10 + (b - b'a')),
        b'A'..=b'F' => Ok(10 + (b - b'A')),
        _ => Err(KeyManagementError::InvalidFormat(
            "invalid hex character".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_passphrase_provider_is_deterministic() {
        let provider = PassphraseKeyProvider::new("secret-passphrase");
        let uuid = [0x33; 16];
        let k1 = provider.load_store_key(uuid).expect("k1");
        let k2 = provider.load_store_key(uuid).expect("k2");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_env_provider_hex_parse() {
        let name = "ERFS_TEST_STORE_KEY_HEX";
        std::env::set_var(
            name,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let provider = EnvKeyProvider::new(name);
        let key = provider.load_store_key([0u8; 16]).expect("env key");
        assert_eq!(key, [0xAA; 32]);
        std::env::remove_var(name);
    }

    #[test]
    fn test_wrapped_file_provider_roundtrip() {
        let dir = TempDir::new().expect("tmp");
        let path = dir.path().join("wrapped.key");
        let store_key = [0x11; 32];
        let wrapping_key = [0x77; 32];
        WrappedFileKeyProvider::write_wrapped_key(&path, store_key, wrapping_key)
            .expect("write wrapped");
        let provider = WrappedFileKeyProvider::new(path, wrapping_key);
        let got = provider.load_store_key([0x55; 16]).expect("load wrapped");
        assert_eq!(got, store_key);
    }
}
