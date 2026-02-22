// SPDX-License-Identifier: Apache-2.0
use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;
use thiserror::Error;

use crate::event::{HlcTimestamp, StreamId};

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Key not found")]
    NotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid key")]
    InvalidKey,
    #[error("Corrupted")]
    Corrupted,
}

#[derive(Debug, Clone)]
pub struct EventHashIndexEntry {
    pub segment_id: [u8; 32],
    pub offset: u64,
}

pub struct EventHashIndex {
    entries: RwLock<HashMap<[u8; 32], EventHashIndexEntry>>,
}

impl Default for EventHashIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHashIndex {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, event_hash: [u8; 32], segment_id: [u8; 32], offset: u64) {
        let mut entries = self.entries.write().unwrap();
        entries.insert(event_hash, EventHashIndexEntry { segment_id, offset });
    }

    pub fn get(&self, event_hash: &[u8; 32]) -> Option<EventHashIndexEntry> {
        let entries = self.entries.read().unwrap();
        entries.get(event_hash).cloned()
    }

    pub fn remove(&self, event_hash: &[u8; 32]) -> Option<EventHashIndexEntry> {
        let mut entries = self.entries.write().unwrap();
        entries.remove(event_hash)
    }

    pub fn len(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    pub fn is_empty(&self) -> bool {
        let entries = self.entries.read().unwrap();
        entries.is_empty()
    }

    pub fn serialize(&self) -> Result<Vec<u8>, IndexError> {
        let entries = self.entries.read().unwrap();
        let mut data = Vec::new();

        for (hash, entry) in entries.iter() {
            data.extend_from_slice(hash);
            data.extend_from_slice(&entry.segment_id);
            data.extend_from_slice(&entry.offset.to_le_bytes());
        }

        Ok(data)
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<(), IndexError> {
        let mut entries = self.entries.write().unwrap();
        entries.clear();

        let entry_size = 32 + 32 + 8;
        if !data.len().is_multiple_of(entry_size) {
            return Err(IndexError::Corrupted);
        }

        for chunk in data.chunks(entry_size) {
            let hash: [u8; 32] = chunk[..32].try_into().unwrap();
            let segment_id: [u8; 32] = chunk[32..64].try_into().unwrap();
            let offset = u64::from_le_bytes(chunk[64..72].try_into().unwrap());

            entries.insert(hash, EventHashIndexEntry { segment_id, offset });
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StreamIndexEntry {
    pub hlc: HlcTimestamp,
    pub segment_id: [u8; 32],
    pub offset: u64,
}

pub struct StreamIndex {
    entries: RwLock<HashMap<StreamId, Vec<StreamIndexEntry>>>,
}

impl StreamIndex {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, stream_id: StreamId, entry: StreamIndexEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.entry(stream_id).or_default().push(entry);
    }

    pub fn get(&self, stream_id: &StreamId) -> Vec<StreamIndexEntry> {
        let entries = self.entries.read().unwrap();
        entries.get(stream_id).cloned().unwrap_or_default()
    }

    pub fn get_tail(&self, stream_id: &StreamId, limit: usize) -> Vec<StreamIndexEntry> {
        let entries = self.entries.read().unwrap();
        if let Some(stream_entries) = entries.get(stream_id) {
            let start = if stream_entries.len() > limit {
                stream_entries.len() - limit
            } else {
                0
            };
            stream_entries[start..].to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn len(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.values().map(|v| v.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        let entries = self.entries.read().unwrap();
        entries.is_empty()
    }
}

impl Default for StreamIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TimeIndexEntry {
    pub min_hlc: HlcTimestamp,
    pub max_hlc: HlcTimestamp,
    pub segment_id: [u8; 32],
}

pub struct TimeIndex {
    entries: RwLock<BTreeMap<HlcTimestamp, TimeIndexEntry>>,
}

impl TimeIndex {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn insert(&self, entry: TimeIndexEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.insert(entry.min_hlc, entry);
    }

    pub fn query_range(
        &self,
        min_hlc: &HlcTimestamp,
        max_hlc: &HlcTimestamp,
    ) -> Vec<TimeIndexEntry> {
        let entries = self.entries.read().unwrap();

        entries
            .range(*min_hlc..=*max_hlc)
            .map(|(_, entry)| entry.clone())
            .collect()
    }

    pub fn get_all(&self) -> Vec<TimeIndexEntry> {
        let entries = self.entries.read().unwrap();
        entries.values().cloned().collect()
    }

    pub fn len(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    pub fn is_empty(&self) -> bool {
        let entries = self.entries.read().unwrap();
        entries.is_empty()
    }
}

impl Default for TimeIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VersionVector {
    versions: BTreeMap<[u8; 16], u64>,
}

impl VersionVector {
    pub fn new() -> Self {
        Self {
            versions: BTreeMap::new(),
        }
    }

    pub fn increment(&mut self, actor_id: [u8; 16]) {
        let counter = self.versions.entry(actor_id).or_insert(0);
        *counter += 1;
    }

    pub fn get(&self, actor_id: &[u8; 16]) -> u64 {
        self.versions.get(actor_id).copied().unwrap_or(0)
    }

    pub fn merge(&mut self, other: &VersionVector) {
        for (actor_id, &count) in other.versions.iter() {
            let self_count = self.versions.entry(*actor_id).or_insert(0);
            *self_count = (*self_count).max(count);
        }
    }

    pub fn dominates(&self, other: &VersionVector) -> bool {
        for (actor_id, &count) in other.versions.iter() {
            if self.versions.get(actor_id).copied().unwrap_or(0) < count {
                return false;
            }
        }
        true
    }

    pub fn to_hashmap(&self) -> std::collections::HashMap<[u8; 16], u64> {
        self.versions.iter().map(|(&k, &v)| (k, v)).collect()
    }
}

impl Default for VersionVector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MaterializedStateEntry {
    pub value: Vec<u8>,
    pub version_vector: VersionVector,
    pub event_hash: [u8; 32],
    pub hlc: HlcTimestamp,
}

pub struct MaterializedStateIndex {
    entries: RwLock<HashMap<Vec<u8>, MaterializedStateEntry>>,
}

impl MaterializedStateIndex {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, key: Vec<u8>, entry: MaterializedStateEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.insert(key, entry);
    }

    pub fn get(&self, key: &[u8]) -> Option<MaterializedStateEntry> {
        let entries = self.entries.read().unwrap();
        entries.get(key).cloned()
    }

    pub fn get_all(&self) -> Vec<(Vec<u8>, MaterializedStateEntry)> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn len(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    pub fn is_empty(&self) -> bool {
        let entries = self.entries.read().unwrap();
        entries.is_empty()
    }
}

impl Default for MaterializedStateIndex {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IndexEngine {
    event_hash_index: EventHashIndex,
    stream_index: StreamIndex,
    time_index: TimeIndex,
    materialized_state_index: MaterializedStateIndex,
}

impl IndexEngine {
    pub fn new() -> Self {
        Self {
            event_hash_index: EventHashIndex::new(),
            stream_index: StreamIndex::new(),
            time_index: TimeIndex::new(),
            materialized_state_index: MaterializedStateIndex::new(),
        }
    }

    pub fn event_hash(&self) -> &EventHashIndex {
        &self.event_hash_index
    }

    pub fn stream(&self) -> &StreamIndex {
        &self.stream_index
    }

    pub fn time(&self) -> &TimeIndex {
        &self.time_index
    }

    pub fn materialized_state(&self) -> &MaterializedStateIndex {
        &self.materialized_state_index
    }
}

impl Default for IndexEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_hash_index() {
        let index = EventHashIndex::new();

        let hash: [u8; 32] = [1u8; 32];
        let segment_id: [u8; 32] = [2u8; 32];

        index.insert(hash, segment_id, 100);

        let entry = index.get(&hash);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().offset, 100);
    }

    #[test]
    fn test_event_hash_index_remove() {
        let index = EventHashIndex::new();

        let hash: [u8; 32] = [1u8; 32];
        index.insert(hash, [2u8; 32], 100);

        let removed = index.remove(&hash);
        assert!(removed.is_some());

        let entry = index.get(&hash);
        assert!(entry.is_none());
    }

    #[test]
    fn test_event_hash_index_len() {
        let index = EventHashIndex::new();

        assert_eq!(index.len(), 0);

        index.insert([1u8; 32], [2u8; 32], 100);
        index.insert([3u8; 32], [4u8; 32], 200);

        assert_eq!(index.len(), 2);
    }

    #[test]
    fn test_event_hash_index_serialize() {
        let index = EventHashIndex::new();

        index.insert([1u8; 32], [2u8; 32], 100);
        index.insert([3u8; 32], [4u8; 32], 200);

        let data = index.serialize().unwrap();

        let index2 = EventHashIndex::new();
        index2.deserialize(&data).unwrap();

        assert_eq!(index2.len(), 2);
    }

    #[test]
    fn test_event_hash_index_deserialize_invalid() {
        let index = EventHashIndex::new();
        let result = index.deserialize(b"invalid");
        assert!(matches!(result, Err(IndexError::Corrupted)));
    }

    #[test]
    fn test_stream_index() {
        let index = StreamIndex::new();

        let stream_id = StreamId::new();

        index.insert(
            stream_id.clone(),
            StreamIndexEntry {
                hlc: HlcTimestamp::now(),
                segment_id: [1u8; 32],
                offset: 100,
            },
        );

        let entries = index.get(&stream_id);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_stream_index_tail() {
        let index = StreamIndex::new();

        let stream_id = StreamId::new();

        for i in 0i64..10 {
            index.insert(
                stream_id.clone(),
                StreamIndexEntry {
                    hlc: HlcTimestamp {
                        physical: i,
                        logical: 0,
                    },
                    segment_id: [1u8; 32],
                    offset: (i * 100) as u64,
                },
            );
        }

        let tail = index.get_tail(&stream_id, 3);
        assert_eq!(tail.len(), 3);
    }

    #[test]
    fn test_time_index() {
        let index = TimeIndex::new();

        index.insert(TimeIndexEntry {
            min_hlc: HlcTimestamp {
                physical: 1000,
                logical: 0,
            },
            max_hlc: HlcTimestamp {
                physical: 1000,
                logical: 0,
            },
            segment_id: [1u8; 32],
        });

        index.insert(TimeIndexEntry {
            min_hlc: HlcTimestamp {
                physical: 2000,
                logical: 0,
            },
            max_hlc: HlcTimestamp {
                physical: 2000,
                logical: 0,
            },
            segment_id: [2u8; 32],
        });

        assert_eq!(index.len(), 2);

        let results = index.query_range(
            &HlcTimestamp {
                physical: 1500,
                logical: 0,
            },
            &HlcTimestamp {
                physical: 2500,
                logical: 0,
            },
        );

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_version_vector() {
        let mut vv = VersionVector::new();

        let actor_id: [u8; 16] = [1u8; 16];

        vv.increment(actor_id);
        vv.increment(actor_id);

        assert_eq!(vv.get(&actor_id), 2);
    }

    #[test]
    fn test_version_vector_merge() {
        let mut vv1 = VersionVector::new();
        vv1.increment([1u8; 16]);
        vv1.increment([1u8; 16]);

        let mut vv2 = VersionVector::new();
        vv2.increment([1u8; 16]);
        vv2.increment([2u8; 16]);

        vv1.merge(&vv2);

        assert_eq!(vv1.get(&[1u8; 16]), 2);
        assert_eq!(vv1.get(&[2u8; 16]), 1);
    }

    #[test]
    fn test_version_vector_dominates() {
        let mut vv1 = VersionVector::new();
        vv1.increment([1u8; 16]);
        vv1.increment([1u8; 16]);
        vv1.increment([2u8; 16]);

        let mut vv2 = VersionVector::new();
        vv2.increment([1u8; 16]);
        vv2.increment([2u8; 16]);

        assert!(vv1.dominates(&vv2));
        assert!(!vv2.dominates(&vv1));
    }

    #[test]
    fn test_materialized_state_index() {
        let index = MaterializedStateIndex::new();

        let key = b"key1".to_vec();

        index.insert(
            key.clone(),
            MaterializedStateEntry {
                value: b"value1".to_vec(),
                version_vector: VersionVector::new(),
                event_hash: [1u8; 32],
                hlc: HlcTimestamp::now(),
            },
        );

        let entry = index.get(&key);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, b"value1");
    }

    #[test]
    fn test_materialized_state_index_get_all() {
        let index = MaterializedStateIndex::new();

        index.insert(
            b"key1".to_vec(),
            MaterializedStateEntry {
                value: b"value1".to_vec(),
                version_vector: VersionVector::new(),
                event_hash: [1u8; 32],
                hlc: HlcTimestamp::now(),
            },
        );

        index.insert(
            b"key2".to_vec(),
            MaterializedStateEntry {
                value: b"value2".to_vec(),
                version_vector: VersionVector::new(),
                event_hash: [2u8; 32],
                hlc: HlcTimestamp::now(),
            },
        );

        let all = index.get_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_index_engine() {
        let engine = IndexEngine::new();

        assert!(engine.event_hash().is_empty());
        assert!(engine.stream().is_empty());
        assert!(engine.time().is_empty());
        assert!(engine.materialized_state().is_empty());
    }

    #[test]
    fn test_time_index_get_all() {
        let index = TimeIndex::new();

        index.insert(TimeIndexEntry {
            min_hlc: HlcTimestamp {
                physical: 1000,
                logical: 0,
            },
            max_hlc: HlcTimestamp {
                physical: 1000,
                logical: 0,
            },
            segment_id: [1u8; 32],
        });

        let all = index.get_all();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_time_index_is_empty() {
        let index = TimeIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_version_vector_new() {
        let vv: VersionVector = VersionVector::new();
        assert_eq!(vv.get(&[0u8; 16]), 0);
    }

    #[test]
    fn test_materialized_state_index_len() {
        let index = MaterializedStateIndex::new();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_stream_index_is_empty() {
        let index = StreamIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }
}
