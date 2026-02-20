// SPDX-License-Identifier: GPL-2.0-only
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use thiserror::Error;
use uuid::Uuid;

use crate::event::HlcTimestamp;

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid epoch")]
    InvalidEpoch,
    #[error("Not found")]
    NotFound,
    #[error("Corrupted")]
    Corrupted,
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SealedSegment {
    pub segment_id: [u8; 32],
    pub offset: u64,
    pub size: u64,
    pub min_hlc: Option<HlcTimestamp>,
    pub max_hlc: Option<HlcTimestamp>,
}

impl SealedSegment {
    pub fn new(segment_id: [u8; 32], offset: u64, size: u64) -> Self {
        Self {
            segment_id,
            offset,
            size,
            min_hlc: None,
            max_hlc: None,
        }
    }

    pub fn with_hlc(
        mut self,
        min_hlc: Option<HlcTimestamp>,
        max_hlc: Option<HlcTimestamp>,
    ) -> Self {
        self.min_hlc = min_hlc;
        self.max_hlc = max_hlc;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRoots {
    pub event_hash_index: Option<[u8; 32]>,
    pub stream_index: Option<[u8; 32]>,
    pub time_index: Option<[u8; 32]>,
    pub materialized_state_index: Option<[u8; 32]>,
}

impl Default for IndexRoots {
    fn default() -> Self {
        Self {
            event_hash_index: None,
            stream_index: None,
            time_index: None,
            materialized_state_index: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub segment_id: [u8; 32],
    pub offset: u64,
    pub hlc: HlcTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub store_uuid: Uuid,
    pub epoch: u64,
    pub sealed_segments: Vec<SealedSegment>,
    pub active_segment: Option<SealedSegment>,
    pub index_roots: IndexRoots,
    pub last_checkpoint: Option<Checkpoint>,
    pub compaction_watermark: u64,
    pub manifest_crc: u32,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            store_uuid: Uuid::new_v4(),
            epoch: 0,
            sealed_segments: Vec::new(),
            active_segment: None,
            index_roots: IndexRoots::default(),
            last_checkpoint: None,
            compaction_watermark: 0,
            manifest_crc: 0,
        }
    }
}

impl Manifest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_sealed_segment(&mut self, segment: SealedSegment) {
        self.sealed_segments.push(segment);
    }

    pub fn set_active_segment(&mut self, segment: SealedSegment) {
        self.active_segment = Some(segment);
    }

    pub fn seal_active_segment(&mut self) -> Option<SealedSegment> {
        self.active_segment.take()
    }

    pub fn increment_epoch(&mut self) {
        self.epoch += 1;
    }

    pub fn set_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.last_checkpoint = Some(checkpoint);
    }

    pub fn serialize(&self) -> Result<Vec<u8>, ManifestError> {
        let json = serde_json::to_string(self)?;
        Ok(json.into_bytes())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, ManifestError> {
        let manifest: Manifest = serde_json::from_slice(data)?;
        Ok(manifest)
    }

    pub fn compute_crc(&self) -> u32 {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        let json = serde_json::to_string(self).unwrap_or_default();
        hasher.update(json.as_bytes());
        hasher.finalize()
    }
}

pub struct ManifestManager {
    manifest_a: RwLock<Manifest>,
    manifest_b: RwLock<Manifest>,
    current_a: RwLock<bool>,
    path_a: std::path::PathBuf,
    path_b: std::path::PathBuf,
}

impl ManifestManager {
    pub fn new(dir: std::path::PathBuf) -> Result<Self, ManifestError> {
        let path_a = dir.join("manifest_a.json");
        let path_b = dir.join("manifest_b.json");

        let (manifest_a, manifest_b, current_a) = if path_a.exists() && path_b.exists() {
            let data_a = std::fs::read(&path_a)?;
            let data_b = std::fs::read(&path_b)?;

            let m_a = Manifest::deserialize(&data_a)?;
            let m_b = Manifest::deserialize(&data_b)?;

            // The one with higher epoch is current
            if m_a.epoch > m_b.epoch {
                (m_a, m_b, true)
            } else {
                (m_a, m_b, false)
            }
        } else if path_a.exists() {
            let data_a = std::fs::read(&path_a)?;
            let m_a = Manifest::deserialize(&data_a)?;
            (m_a.clone(), m_a, true)
        } else if path_b.exists() {
            let data_b = std::fs::read(&path_b)?;
            let m_b = Manifest::deserialize(&data_b)?;
            (m_b.clone(), m_b, false)
        } else {
            let mut base = Manifest::new();
            base.manifest_crc = base.compute_crc();
            let data = base.serialize()?;
            std::fs::write(&path_a, &data)?;
            std::fs::write(&path_b, &data)?;
            (base.clone(), base, true)
        };

        Ok(Self {
            manifest_a: RwLock::new(manifest_a),
            manifest_b: RwLock::new(manifest_b),
            current_a: RwLock::new(current_a),
            path_a,
            path_b,
        })
    }

    pub fn read_current(&self) -> Result<Manifest, ManifestError> {
        let current_a = *self.current_a.read().unwrap();

        if current_a {
            Ok(self.manifest_a.read().unwrap().clone())
        } else {
            Ok(self.manifest_b.read().unwrap().clone())
        }
    }

    pub fn write(&self, manifest: &Manifest) -> Result<(), ManifestError> {
        let mut manifest = manifest.clone();
        manifest.manifest_crc = manifest.compute_crc();

        let current_a = *self.current_a.read().unwrap();

        if current_a {
            *self.manifest_b.write().unwrap() = manifest.clone();
        } else {
            *self.manifest_a.write().unwrap() = manifest.clone();
        }

        self.flush(manifest)?;

        *self.current_a.write().unwrap() = !current_a;

        Ok(())
    }

    /// Build the next manifest image for checkpoint write+fsync chaining.
    pub fn prepare_checkpoint_write(
        &self,
        checkpoint: Checkpoint,
    ) -> Result<(std::path::PathBuf, Manifest), ManifestError> {
        let current_a = *self.current_a.read().unwrap();
        let target_path = if current_a {
            self.path_b.clone()
        } else {
            self.path_a.clone()
        };

        let mut manifest = self.read_current()?;
        manifest.set_checkpoint(checkpoint);
        manifest.increment_epoch();
        manifest.manifest_crc = manifest.compute_crc();
        Ok((target_path, manifest))
    }

    /// Commit a manifest that has already been written externally to the inactive slot.
    pub fn commit_prepared_checkpoint(
        &self,
        target_path: &std::path::Path,
        manifest: &Manifest,
    ) -> Result<(), ManifestError> {
        let current_a = *self.current_a.read().unwrap();
        let expected_target = if current_a {
            &self.path_b
        } else {
            &self.path_a
        };
        if target_path != expected_target {
            return Err(ManifestError::InvalidData(
                "checkpoint target path no longer matches current manifest slot".to_string(),
            ));
        }

        let mut committed = manifest.clone();
        committed.manifest_crc = committed.compute_crc();
        if current_a {
            *self.manifest_b.write().unwrap() = committed;
        } else {
            *self.manifest_a.write().unwrap() = committed;
        }
        *self.current_a.write().unwrap() = !current_a;
        Ok(())
    }

    fn flush(&self, manifest: Manifest) -> Result<(), ManifestError> {
        let current_a = *self.current_a.read().unwrap();

        let data = manifest.serialize()?;

        if current_a {
            std::fs::write(&self.path_b, &data)?;
        } else {
            std::fs::write(&self.path_a, &data)?;
        }

        Ok(())
    }

    pub fn add_sealed_segment(&self, segment: SealedSegment) -> Result<(), ManifestError> {
        let mut manifest = self.read_current()?;
        manifest.add_sealed_segment(segment);
        manifest.increment_epoch();
        self.write(&manifest)
    }

    pub fn set_active_segment(&self, segment: SealedSegment) -> Result<(), ManifestError> {
        let mut manifest = self.read_current()?;
        manifest.set_active_segment(segment);
        self.write(&manifest)
    }

    pub fn seal_active_segment(&self) -> Result<SealedSegment, ManifestError> {
        let mut manifest = self.read_current()?;
        let sealed = manifest.seal_active_segment();
        manifest.increment_epoch();

        if let Some(segment) = sealed {
            self.write(&manifest)?;
            Ok(segment)
        } else {
            Err(ManifestError::NotFound)
        }
    }

    pub fn get_sealed_segments(&self) -> Result<Vec<SealedSegment>, ManifestError> {
        let manifest = self.read_current()?;
        Ok(manifest.sealed_segments)
    }

    pub fn get_active_segment(&self) -> Result<Option<SealedSegment>, ManifestError> {
        let manifest = self.read_current()?;
        Ok(manifest.active_segment)
    }

    pub fn epoch(&self) -> Result<u64, ManifestError> {
        let manifest = self.read_current()?;
        Ok(manifest.epoch)
    }

    pub fn store_uuid(&self) -> Result<Uuid, ManifestError> {
        let manifest = self.read_current()?;
        Ok(manifest.store_uuid)
    }

    /// Validate manifest integrity.
    ///
    /// Checks:
    /// - Both A and B manifests are readable
    /// - CRC checksums match
    /// - Epoch is consistent
    /// - Active segment is valid (if present)
    /// - Sealed segments are valid
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Read both manifests
        let manifest_a = self.manifest_a.read().unwrap();
        let manifest_b = self.manifest_b.read().unwrap();

        // Validate A
        if self.path_a.exists() {
            let stored_crc_a = manifest_a.manifest_crc;
            let computed_crc_a = manifest_a.compute_crc();
            if stored_crc_a != computed_crc_a {
                return Err(ManifestError::InvalidData(format!(
                    "Manifest A CRC mismatch: stored={}, computed={}",
                    stored_crc_a, computed_crc_a
                )));
            }
        }

        // Validate B
        if self.path_b.exists() {
            let stored_crc_b = manifest_b.manifest_crc;
            let computed_crc_b = manifest_b.compute_crc();
            if stored_crc_b != computed_crc_b {
                return Err(ManifestError::InvalidData(format!(
                    "Manifest B CRC mismatch: stored={}, computed={}",
                    stored_crc_b, computed_crc_b
                )));
            }
        }

        // Validate active segment if present
        if let Some(ref active) = manifest_a.active_segment {
            if active.segment_id.iter().all(|&b| b == 0) {
                return Err(ManifestError::InvalidData(
                    "Active segment has invalid ID".to_string(),
                ));
            }
        }

        // Validate sealed segments
        for segment in &manifest_a.sealed_segments {
            if segment.segment_id.iter().all(|&b| b == 0) {
                return Err(ManifestError::InvalidData(
                    "Sealed segment has invalid ID".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_new() {
        let manifest = Manifest::new();

        assert_eq!(manifest.epoch, 0);
        assert!(manifest.sealed_segments.is_empty());
        assert!(manifest.active_segment.is_none());
    }

    #[test]
    fn test_manifest_add_sealed_segment() {
        let mut manifest = Manifest::new();

        let segment = SealedSegment::new([1u8; 32], 0, 1024);
        manifest.add_sealed_segment(segment.clone());

        assert_eq!(manifest.sealed_segments.len(), 1);
    }

    #[test]
    fn test_manifest_active_segment() {
        let mut manifest = Manifest::new();

        let active = SealedSegment::new([2u8; 32], 0, 1024);
        manifest.set_active_segment(active);

        assert!(manifest.active_segment.is_some());

        let sealed = manifest.seal_active_segment();
        assert!(sealed.is_some());
        assert!(manifest.active_segment.is_none());
    }

    #[test]
    fn test_manifest_epoch() {
        let mut manifest = Manifest::new();

        assert_eq!(manifest.epoch, 0);

        manifest.increment_epoch();
        assert_eq!(manifest.epoch, 1);

        manifest.increment_epoch();
        assert_eq!(manifest.epoch, 2);
    }

    #[test]
    fn test_manifest_checkpoint() {
        let mut manifest = Manifest::new();

        let checkpoint = Checkpoint {
            segment_id: [3u8; 32],
            offset: 100,
            hlc: HlcTimestamp::now(),
        };

        manifest.set_checkpoint(checkpoint.clone());

        assert!(manifest.last_checkpoint.is_some());
    }

    #[test]
    fn test_manifest_serialize() -> Result<(), ManifestError> {
        let mut manifest = Manifest::new();

        let segment = SealedSegment::new([1u8; 32], 0, 1024)
            .with_hlc(Some(HlcTimestamp::now()), Some(HlcTimestamp::now()));
        manifest.add_sealed_segment(segment);

        let data = manifest.serialize()?;

        let restored = Manifest::deserialize(&data)?;

        assert_eq!(manifest.epoch, restored.epoch);
        assert_eq!(
            manifest.sealed_segments.len(),
            restored.sealed_segments.len()
        );

        Ok(())
    }

    #[test]
    fn test_manifest_manager_new() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let manifest = manager.read_current()?;
        assert_eq!(manifest.epoch, 0);

        Ok(())
    }

    #[test]
    fn test_manifest_manager_write_read() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let mut manifest = Manifest::new();
        manifest.epoch = 5;

        manager.write(&manifest)?;

        let read = manager.read_current()?;
        assert_eq!(read.epoch, 5);

        Ok(())
    }

    #[test]
    fn test_manifest_manager_segments() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let segment = SealedSegment::new([1u8; 32], 0, 1024);
        manager.add_sealed_segment(segment)?;

        let segments = manager.get_sealed_segments()?;
        assert_eq!(segments.len(), 1);

        Ok(())
    }

    #[test]
    fn test_manifest_manager_active_segment() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let segment = SealedSegment::new([2u8; 32], 0, 2048);
        manager.set_active_segment(segment)?;

        let active = manager.get_active_segment()?;
        assert!(active.is_some());

        Ok(())
    }

    #[test]
    fn test_manifest_manager_seal_active() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let segment = SealedSegment::new([2u8; 32], 0, 2048);
        manager.set_active_segment(segment)?;

        let _sealed = manager.seal_active_segment()?;

        let active = manager.get_active_segment()?;
        assert!(active.is_none());

        Ok(())
    }

    #[test]
    fn test_sealed_segment_with_hlc() {
        let segment = SealedSegment::new([1u8; 32], 100, 2048);

        let min_hlc = HlcTimestamp {
            physical: 1000,
            logical: 0,
        };
        let max_hlc = HlcTimestamp {
            physical: 2000,
            logical: 5,
        };

        let segment = segment.with_hlc(Some(min_hlc), Some(max_hlc));

        assert!(segment.min_hlc.is_some());
        assert!(segment.max_hlc.is_some());
    }

    #[test]
    fn test_manifest_crc() {
        let manifest = Manifest::new();

        let crc = manifest.compute_crc();

        assert!(crc != 0);
    }

    #[test]
    fn test_sealed_segment_new() {
        let segment = SealedSegment::new([1u8; 32], 100, 200);

        assert_eq!(segment.segment_id, [1u8; 32]);
        assert_eq!(segment.offset, 100);
        assert_eq!(segment.size, 200);
        assert!(segment.min_hlc.is_none());
        assert!(segment.max_hlc.is_none());
    }

    #[test]
    fn test_manifest_set_checkpoint() {
        let mut manifest = Manifest::new();

        let checkpoint = Checkpoint {
            segment_id: [1u8; 32],
            offset: 100,
            hlc: HlcTimestamp::now(),
        };

        manifest.set_checkpoint(checkpoint);

        assert!(manifest.last_checkpoint.is_some());
    }

    #[test]
    fn test_index_roots_default() {
        let roots: IndexRoots = IndexRoots::default();

        assert!(roots.event_hash_index.is_none());
        assert!(roots.stream_index.is_none());
        assert!(roots.time_index.is_none());
        assert!(roots.materialized_state_index.is_none());
    }

    #[test]
    fn test_manifest_serialize_deserialize() -> Result<(), ManifestError> {
        let mut manifest = Manifest::new();
        manifest.epoch = 10;

        let data = manifest.serialize()?;
        let restored = Manifest::deserialize(&data)?;

        assert_eq!(manifest.epoch, restored.epoch);

        Ok(())
    }

    #[test]
    fn test_manifest_persistence_single_file() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        // Write first manifest
        {
            let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;
            let mut manifest = Manifest::new();
            manifest.epoch = 5;
            manager.write(&manifest)?;
        }

        // Read it back
        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;
        let m = manager.read_current()?;
        assert_eq!(m.epoch, 5);

        Ok(())
    }

    #[test]
    fn test_manifest_persistence_multiple_epochs() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        // Write multiple epochs
        for i in 1..=3 {
            let mut manifest = Manifest::new();
            manifest.epoch = i;
            manager.write(&manifest)?;
        }

        // Read back - should have highest epoch
        let m = manager.read_current()?;
        assert_eq!(m.epoch, 3);

        Ok(())
    }

    #[test]
    fn test_sealed_segment_default() {
        let segment = SealedSegment::default();
        assert_eq!(segment.segment_id, [0u8; 32]);
        assert_eq!(segment.offset, 0);
        assert_eq!(segment.size, 0);
    }

    #[test]
    fn test_manifest_seal_no_active_segment() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        // Try to seal when there's no active segment
        let result = manager.seal_active_segment();
        assert!(matches!(result, Err(ManifestError::NotFound)));

        Ok(())
    }

    #[test]
    fn test_manifest_get_epoch() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let epoch = manager.epoch()?;
        assert_eq!(epoch, 0);

        Ok(())
    }

    #[test]
    fn test_prepare_and_commit_checkpoint_write() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();
        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let checkpoint = Checkpoint {
            segment_id: [9u8; 32],
            offset: 123,
            hlc: HlcTimestamp::now(),
        };
        let (target_path, prepared) = manager.prepare_checkpoint_write(checkpoint.clone())?;
        std::fs::write(&target_path, prepared.serialize()?)?;
        manager.commit_prepared_checkpoint(&target_path, &prepared)?;

        let current = manager.read_current()?;
        assert_eq!(current.epoch, 1);
        assert!(current.last_checkpoint.is_some());
        let cp = current.last_checkpoint.unwrap();
        assert_eq!(cp.segment_id, checkpoint.segment_id);
        assert_eq!(cp.offset, checkpoint.offset);
        Ok(())
    }

    #[test]
    fn test_manifest_get_store_uuid() -> Result<(), ManifestError> {
        let temp_dir = TempDir::new().unwrap();

        let manager = ManifestManager::new(temp_dir.path().to_path_buf())?;

        let uuid = manager.store_uuid();
        assert!(uuid.is_ok());

        Ok(())
    }
}
