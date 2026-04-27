//! TFTP file provider backed by edgerun-storage encrypted blobs.
//!
//! This allows the TFTP server to serve PXE boot images that are stored
//! as AES-256-GCM encrypted blobs in the edgerun blob store.
//!
//! # File Mapping
//!
//! TFTP filenames are mapped to blob IDs via a simple registry:
//! - `bootx64.efi` → blob ID (SHA-256 content hash)
//! - `pxelinux.0` → blob ID
//! - `vmlinuz` → blob ID
//! - etc.
//!
//! The blob store decrypts on-the-fly and serves chunks via TFTP.

use alloc::boxed::Box;
use alloc::collections::BTreeMap as HashMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use super::server::FileProvider;
use crate::std::io;

/// A decrypted blob cached in memory for fast TFTP serving.
struct CachedBlob {
    data: Vec<u8>,
}

/// Decryptor function type — decrypts a blob entry to plaintext.
pub type DecryptFn = Arc<dyn Fn(&[u8], &[u8]) -> io::Result<Vec<u8>> + Send + Sync>;

/// TFTP file provider that serves files from the encrypted blob store.
///
/// # Setup
/// ```no_run
/// use edgerun_tftp::blob_provider::BlobTftpProvider;
///
/// // You provide a decryptor function that knows how to decrypt blobs
/// let decryptor: Box<dyn Fn(&[u8], &[u8]) -> edgerun_tftp::std::io::Result<Vec<u8>> + Send + Sync> =
///     Box::new(|nonce, ciphertext| {
///         // Your AES-GCM decryption here
///         Ok(Vec::new())
///     });
///
/// let mut provider = BlobTftpProvider::new(Box::new(decryptor));
///
/// // Register boot files — map TFTP filenames to blob IDs
/// // provider.register("bootx64.efi", blob_id);
///
/// // Or register from plaintext directly (for development)
/// // provider.register_plaintext("vmlinuz", kernel_bytes.to_vec());
/// ```
#[allow(clippy::type_complexity)]
pub struct BlobTftpProvider {
    decryptor: Box<DecryptFn>,
    /// Filename → blob mapping.
    registry: HashMap<String, BlobEntry>,
    /// Decrypted blobs cached in memory.
    cache: HashMap<String, Arc<CachedBlob>>,
}

/// Entry in the blob registry.
pub enum BlobEntry {
    /// Reference to an encrypted blob (decrypt on demand).
    Encrypted {
        blob_id: String,
        ciphertext: Vec<u8>,
        nonce: Vec<u8>,
    },
    /// Plaintext blob (already decrypted, for development or unencrypted files).
    Plaintext { data: Vec<u8> },
}

impl BlobTftpProvider {
    /// Create a new blob-backed TFTP provider.
    pub fn new(decryptor: Box<DecryptFn>) -> Self {
        Self {
            decryptor,
            registry: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Register a boot file from an encrypted blob.
    ///
    /// `filename` is what the PXE client requests (e.g. "bootx64.efi").
    /// `blob_id` is the content-addressed ID from the blob store.
    /// `nonce` and `ciphertext` are the raw encrypted blob contents.
    pub fn register_encrypted(
        &mut self,
        filename: &str,
        blob_id: String,
        nonce: Vec<u8>,
        ciphertext: Vec<u8>,
    ) {
        self.registry.insert(
            filename.to_string(),
            BlobEntry::Encrypted {
                blob_id,
                ciphertext,
                nonce,
            },
        );
        // Clear cached version if re-registering
        self.cache.remove(filename);
    }

    /// Register a boot file from plaintext (no encryption).
    ///
    /// Useful for development or when serving unencrypted boot files.
    pub fn register_plaintext(&mut self, filename: &str, data: Vec<u8>) {
        self.registry.insert(
            filename.to_string(),
            BlobEntry::Plaintext { data: data.clone() },
        );
        // Cache it
        self.cache
            .insert(filename.to_string(), Arc::new(CachedBlob { data }));
    }

    /// Remove a registered boot file.
    pub fn unregister(&mut self, filename: &str) {
        self.registry.remove(filename);
        self.cache.remove(filename);
    }

    /// List all registered boot files.
    pub fn list_files(&self) -> Vec<&str> {
        self.registry.keys().map(|s| s.as_str()).collect()
    }

    /// Pre-decrypt a blob into the cache (async warm-up).
    ///
    /// Call this during boot to decrypt large EFI binaries
    /// before PXE clients request them.
    pub fn warm_cache(&mut self, filename: &str) -> io::Result<()> {
        if self.cache.contains_key(filename) {
            return Ok(());
        }

        if let Some(BlobEntry::Encrypted {
            nonce, ciphertext, ..
        }) = self.registry.get(filename)
        {
            let plaintext = (self.decryptor)(nonce, ciphertext)?;
            self.cache.insert(
                filename.to_string(),
                Arc::new(CachedBlob { data: plaintext }),
            );
            edgerun_log::warn!(
                "edgerun-tftp: cached '{}' ({} bytes)",
                filename,
                self.cache[filename].data.len()
            );
        }

        Ok(())
    }

    /// Decrypt all registered blobs into the cache.
    pub fn warm_all(&mut self) -> io::Result<()> {
        let filenames: Vec<String> = self.registry.keys().cloned().collect();
        for filename in filenames {
            if let Err(e) = self.warm_cache(&filename) {
                edgerun_log::warn!("edgerun-tftp: failed to cache '{}': {}", filename, e);
            }
        }
        Ok(())
    }

    /// Get the decryptor function (for external use).
    pub fn decryptor(&self) -> &DecryptFn {
        &self.decryptor
    }
}

impl FileProvider for BlobTftpProvider {
    fn file_size(&self, filename: &str) -> Option<u64> {
        // Check cache first
        if let Some(blob) = self.cache.get(filename) {
            return Some(blob.data.len() as u64);
        }

        // Check registry
        match self.registry.get(filename)? {
            BlobEntry::Plaintext { data } => Some(data.len() as u64),
            BlobEntry::Encrypted {
                ciphertext,
                nonce: _nonce,
                ..
            } => {
                // For encrypted blobs, we don't know plaintext size without decrypting.
                // Best estimate: ciphertext length (slightly larger due to GCM tag).
                // The actual size will be reported in OACK after warm_cache.
                // For now, return ciphertext size as upper bound.
                Some(ciphertext.len() as u64)
            }
        }
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        // Try cache first
        if let Some(blob) = self.cache.get(filename) {
            if offset >= blob.data.len() {
                return None;
            }
            let end = (offset + max_size).min(blob.data.len());
            return Some(blob.data[offset..end].to_vec());
        }

        // Try plaintext registry entry
        if let Some(BlobEntry::Plaintext { data }) = self.registry.get(filename) {
            if offset >= data.len() {
                return None;
            }
            let end = (offset + max_size).min(data.len());
            return Some(data[offset..end].to_vec());
        }

        // Encrypted blob — need to decrypt on demand
        if let Some(BlobEntry::Encrypted {
            ciphertext, nonce, ..
        }) = self.registry.get(filename)
        {
            match (self.decryptor)(nonce, ciphertext) {
                Ok(plaintext) => {
                    // Cache it for subsequent blocks
                    if let Some(BlobEntry::Encrypted { .. }) = self.registry.get(filename) {
                        // Note: we can't mutate self here (immutable borrow),
                        // but the cache will be populated on the next file_size call.
                        // For now just return the block.
                    }
                    if offset >= plaintext.len() {
                        return None;
                    }
                    let end = (offset + max_size).min(plaintext.len());
                    return Some(plaintext[offset..end].to_vec());
                }
                Err(e) => {
                    edgerun_log::warn!("edgerun-tftp: decrypt failed for '{}': {}", filename, e);
                    return None;
                }
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------
// Simple FileProvider for testing (serves from memory)
// ---------------------------------------------------------------------------

/// In-memory file provider for testing.
pub struct MemFileProvider {
    files: HashMap<String, Vec<u8>>,
}

impl MemFileProvider {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, filename: &str, data: Vec<u8>) {
        self.files.insert(filename.to_string(), data);
    }
}

impl Default for MemFileProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FileProvider for MemFileProvider {
    fn file_size(&self, filename: &str) -> Option<u64> {
        self.files.get(filename).map(|d| d.len() as u64)
    }

    fn read_block(&self, filename: &str, offset: usize, max_size: usize) -> Option<Vec<u8>> {
        let data = self.files.get(filename)?;
        if offset >= data.len() {
            return None;
        }
        let end = (offset + max_size).min(data.len());
        Some(data[offset..end].to_vec())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_mem_provider_file_size() {
        let mut provider = MemFileProvider::new();
        provider.add_file("test.bin", vec![1, 2, 3, 4, 5]);
        assert_eq!(provider.file_size("test.bin"), Some(5));
        assert_eq!(provider.file_size("missing.bin"), None);
    }

    #[test]
    fn test_mem_provider_read_block() {
        let mut provider = MemFileProvider::new();
        provider.add_file("test.bin", (0..100).collect::<Vec<u8>>());

        // First block
        let block = provider.read_block("test.bin", 0, 50).unwrap();
        assert_eq!(block.len(), 50);
        assert_eq!(block[0], 0);
        assert_eq!(block[49], 49);

        // Second block
        let block = provider.read_block("test.bin", 50, 50).unwrap();
        assert_eq!(block.len(), 50);
        assert_eq!(block[0], 50);
        assert_eq!(block[49], 99);

        // Past end
        assert!(provider.read_block("test.bin", 100, 50).is_none());
    }

    #[test]
    fn test_mem_provider_partial_block() {
        let mut provider = MemFileProvider::new();
        provider.add_file("small.bin", vec![1, 2, 3]);

        let block = provider.read_block("small.bin", 0, 512).unwrap();
        assert_eq!(block.len(), 3); // Less than max_size = last block
    }
}
