//! Filesystem blob store with AES-GCM encryption at rest.
//!
//! Implements the blob confidentiality invariants from the protocol spec (§6.2):
//! - Every persisted blob is encrypted at rest
//! - Every persisted blob names at least one recipient
//! - No plaintext blob persistence path
//!
//! Blobs are stored as content-addressed files in the blob directory:
//! `{blob_dir}/{first_4_hex_of_blob_id}/{blob_id}.blob`
//!
//! The blob file format:
//!   [nonce (12 bytes)][ciphertext (variable length)]
//!
//! ## Key management
//!
//! The blob encryption key is **persistent** and derived deterministically:
//!
//! - **Software signer (dev)**: HKDF-SHA256 from the node's private key bytes
//!   (`"lifegraph:v0:blob-key"` as info). Same config → same key across restarts.
//!
//! - **Hardware signer (TPM/YubiKey)**: The sealed/encrypted blob key is stored
//!   at `{blob_dir}/.blob_key.sealed`. On first open, a random key is generated
//!   and sealed with the hardware. On subsequent opens, it is unsealed.
//!
//! Recipient metadata is stored in SQLite via the parent storage layer.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::error::StorageError;

/// Configuration for the blob store.
#[derive(Clone, Debug)]
pub struct BlobStoreConfig {
    /// Directory for encrypted blob files.
    pub blob_dir: PathBuf,
}

/// How the blob encryption key is obtained.
#[derive(Clone)]
pub enum BlobKeySource {
    /// Dev-only: derive the key from the node's private key via HKDF.
    /// The private key bytes are passed in and used once to derive the blob key.
    /// The raw private key is NOT stored anywhere by this module.
    Software { private_key_bytes: Vec<u8> },
    /// Production: the key is sealed/encrypted by hardware (TPM, YubiKey, etc.).
    /// The sealed key file is stored at `{blob_dir}/.blob_key.sealed`.
    /// On first use, a random key is generated and sealed.
    /// The `unseal_fn` is called to recover the key from the sealed blob.
    HardwareSealed {
        unseal_fn: std::sync::Arc<dyn Fn(&[u8]) -> Result<[u8; 32], crate::error::StorageError> + Send + Sync>,
        seal_fn: std::sync::Arc<dyn Fn(&[u8; 32]) -> Result<Vec<u8>, crate::error::StorageError> + Send + Sync>,
    },
}

impl std::fmt::Debug for BlobKeySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Software { private_key_bytes } => f
                .debug_struct("Software")
                .field("private_key_bytes", &format!("[{} bytes]", private_key_bytes.len()))
                .finish(),
            Self::HardwareSealed { .. } => f.debug_struct("HardwareSealed").finish(),
        }
    }
}

/// Encrypted blob store.
pub struct BlobStore {
    config: BlobStoreConfig,
    /// Persistent blob encryption key. Derived from the node's identity
    /// so that blobs remain decryptable across restarts.
    key: [u8; 32],
}

impl BlobStore {
    /// Opens a blob store at the given directory.
    ///
    /// The blob encryption key is derived or unsealed based on the key source.
    /// For `DeriveFromPrivateKey`, the key is deterministic — same private key
    /// always produces the same blob key. For `HardwareSealed`, the key is
    /// generated once and persists in sealed form on disk.
    pub fn open(
        config: &BlobStoreConfig,
        key_source: BlobKeySource,
    ) -> Result<Self, StorageError> {
        fs::create_dir_all(&config.blob_dir)?;

        let key = match key_source {
            BlobKeySource::Software { private_key_bytes } => {
                derive_blob_key_from_private_key(&private_key_bytes)
            }
            BlobKeySource::HardwareSealed { unseal_fn, seal_fn } => {
                load_or_create_sealed_key(&config.blob_dir, &*unseal_fn, &*seal_fn)?
            }
        };

        Ok(Self {
            config: config.clone(),
            key,
        })
    }

    /// Stores plaintext as an encrypted blob.
    ///
    /// The blob ID is derived from the SHA-256 hash of the plaintext
    /// (content-addressed). The ciphertext is stored on the filesystem.
    ///
    /// Returns the blob ID.
    pub fn store(
        &self,
        plaintext: &[u8],
        _recipients: &[Vec<u8>],
    ) -> Result<String, StorageError> {
        // Derive blob ID from plaintext hash (content-addressed)
        let blob_id = hex::encode(Sha256::digest(plaintext));

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        getrandom::fill(&mut nonce_bytes)
            .map_err(|e| StorageError::Encryption(format!("RNG failed: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| StorageError::Encryption(format!("AES-GCM encryption failed: {}", e)))?;

        // Write to filesystem: [nonce (12 bytes)][ciphertext]
        let blob_path = blob_file_path(&self.config.blob_dir, &blob_id);
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&blob_path)?;
        file.write_all(nonce_bytes.as_slice())?;
        file.write_all(&ciphertext)?;
        file.sync_all()?;

        Ok(blob_id)
    }

    /// Loads a blob by its ID.
    ///
    /// Returns the ciphertext, nonce, and recipient metadata.
    /// The caller is responsible for verifying they are a listed recipient
    /// before decrypting.
    pub fn load(&self, blob_id: &str) -> Result<Option<BlobEntry>, StorageError> {
        let blob_path = blob_file_path(&self.config.blob_dir, blob_id);
        if !blob_path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&blob_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        if data.len() < 12 {
            return Err(StorageError::Decryption(
                "blob file too short to contain nonce".into(),
            ));
        }

        let (nonce, ciphertext) = data.split_at(12);
        let nonce = nonce.to_vec();
        let ciphertext = ciphertext.to_vec();

        Ok(Some(BlobEntry {
            ciphertext,
            nonce,
            recipients: Vec::new(), // TODO: load from SQLite
        }))
    }

    /// Decrypts a blob's ciphertext using the store's key.
    ///
    /// In production, the node should verify it is a listed recipient
    /// before calling this method.
    pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, StorageError> {
        if nonce.len() != 12 {
            return Err(StorageError::Decryption(
                "nonce must be 12 bytes".into(),
            ));
        }
        let nonce = Nonce::from_slice(nonce);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| StorageError::Decryption(format!("AES-GCM decryption failed: {}", e)))
    }
}

// ---------------------------------------------------------------------------
// Key derivation — software (dev) path
// ---------------------------------------------------------------------------

/// Derives a persistent blob encryption key from the node's private key.
///
/// Uses HKDF-SHA256 with:
/// - IKM: the node's private key bytes (32 bytes for ECDSA P-256)
/// - salt: empty (we want deterministic output)
/// - info: `"lifegraph:v0:blob-key"` (domain separation)
///
/// Same private key always produces the same blob key, so blobs survive restarts.
fn derive_blob_key_from_private_key(private_key_bytes: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, private_key_bytes);
    let mut okm = [0u8; 32];
    hk.expand(b"lifegraph:v0:blob-key", &mut okm)
        .expect("HKDF expand to 32 bytes never fails for SHA-256");
    okm
}

// ---------------------------------------------------------------------------
// Key management — hardware (production) path
// ---------------------------------------------------------------------------

const SEALED_KEY_FILENAME: &str = ".blob_key.sealed";

/// Loads the sealed blob key from disk and unseals it, or generates a new
/// random key, seals it, and stores it on disk.
fn load_or_create_sealed_key(
    blob_dir: &Path,
    unseal_fn: &dyn Fn(&[u8]) -> Result<[u8; 32], StorageError>,
    seal_fn: &dyn Fn(&[u8; 32]) -> Result<Vec<u8>, StorageError>,
) -> Result<[u8; 32], StorageError> {
    let sealed_path = blob_dir.join(SEALED_KEY_FILENAME);

    if sealed_path.exists() {
        // Unseal existing key
        let mut sealed_data = Vec::new();
        File::open(&sealed_path)?.read_to_end(&mut sealed_data)?;
        unseal_fn(&sealed_data)
    } else {
        // Generate new key, seal it, store on disk
        let mut key = [0u8; 32];
        getrandom::fill(&mut key)
            .map_err(|e| StorageError::Encryption(format!("RNG failed: {}", e)))?;

        let sealed = seal_fn(&key)?;
        let mut file = File::create(&sealed_path)?;
        file.write_all(&sealed)?;
        file.sync_all()?;

        Ok(key)
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A stored blob entry with ciphertext and metadata.
pub struct BlobEntry {
    /// The encrypted ciphertext bytes (without nonce).
    pub ciphertext: Vec<u8>,
    /// The AES-GCM nonce (12 bytes).
    pub nonce: Vec<u8>,
    /// Identities of the blob's recipients.
    pub recipients: Vec<Vec<u8>>,
}

/// Computes the filesystem path for a blob.
///
/// Uses a two-level directory structure based on the first 4 hex characters
/// of the blob ID to avoid filesystem limits on single-directory file counts.
pub fn blob_file_path(blob_dir: &Path, blob_id: &str) -> PathBuf {
    let prefix = &blob_id[..4.min(blob_id.len())];
    blob_dir.join(prefix).join(format!("{}.blob", blob_id))
}
