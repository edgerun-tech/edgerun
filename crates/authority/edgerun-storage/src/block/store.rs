use crate::prelude::v1::*;
use std::collections::BTreeMap;

use edgerun_protocols::core_protocol::protocol::EventEnvelope;

use crate::core::{EventLocation, EventLog, ScannedEvent};
use crate::error::StorageError;

use super::{BlockEventLog, BlockStorage};

/// Rebuildable stream index over a block-device-backed append log.
///
/// The block log stores already-signed events; this struct keeps in-memory
/// indexes for fast lookup of latest heads and seq→offset locations after
/// recovery. It does not assign sequence numbers or validate hash chains.
pub struct BlockStreamStore<S: BlockStorage> {
    event_log: BlockEventLog<S>,
    heads: BTreeMap<Vec<u8>, (u64, Vec<u8>)>,
    locations: BTreeMap<Vec<u8>, BTreeMap<u64, EventLocation>>,
}

impl<S: BlockStorage> BlockStreamStore<S> {
    pub fn open(device: S) -> Result<Self, StorageError> {
        let event_log = BlockEventLog::open(device)?;
        let scanned = event_log.scan()?;

        let mut heads: BTreeMap<Vec<u8>, (u64, Vec<u8>)> = BTreeMap::new();
        let mut locations: BTreeMap<Vec<u8>, BTreeMap<u64, EventLocation>> = BTreeMap::new();

        for ScannedEvent { location, event: _ } in scanned {
            let stream = location.stream_id.clone();
            locations
                .entry(stream.clone())
                .or_default()
                .insert(location.seq, location.clone());
            heads.insert(stream, (location.seq, location.event_hash));
        }

        Ok(Self {
            event_log,
            heads,
            locations,
        })
    }

    pub fn append_event(&mut self, event: EventEnvelope) -> Result<EventLocation, StorageError> {
        let event_hash = crate::core::canonical_event_hash(&event).value.to_vec();
        let receipt = self.event_log.append_event(&event)?;
        let location = EventLocation {
            stream_id: event.stream_id.clone(),
            seq: receipt.seq,
            event_hash,
            file_offset: receipt.file_offset,
            envelope_version: event.envelope_version,
        };

        self.locations
            .entry(event.stream_id.clone())
            .or_default()
            .insert(receipt.seq, location.clone());
        self.heads.insert(
            event.stream_id.clone(),
            (receipt.seq, location.event_hash.clone()),
        );

        Ok(location)
    }

    pub fn get_event(
        &self,
        stream_id: &[u8],
        seq: u64,
    ) -> Result<Option<EventEnvelope>, StorageError> {
        let seq_index = self
            .locations
            .get(stream_id)
            .and_then(|index| index.get(&seq));
        let Some(location) = seq_index else {
            return Ok(None);
        };

        self.event_log.read_event(stream_id, seq, location)
    }

    pub fn get_head(&self, stream_id: &[u8]) -> Option<(u64, Vec<u8>)> {
        self.heads.get(stream_id).cloned()
    }

    pub fn scan(&self) -> Result<Vec<ScannedEvent>, StorageError> {
        self.event_log.scan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::InMemoryBlockDevice;
    use edgerun_protocols::core_protocol::protocol::Digest;

    fn event(stream_id: &[u8], seq: u64, prev_hash: Option<Vec<u8>>) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            event_version: 1,
            stream_id: stream_id.to_vec(),
            seq,
            prev_event_hash: prev_hash.map(|value| Digest {
                algorithm: 1,
                value,
            }),
            event_type: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
            ..Default::default()
        }
    }

    #[test]
    fn block_stream_store_replays_and_rebuilds_head() {
        let device = InMemoryBlockDevice::new(32, 64);
        let mut store = BlockStreamStore::open(device.clone()).unwrap();
        let hash0 = {
            let e0 = event(b"stream", 0, None);
            crate::core::canonical_event_hash(&e0).value
        };
        let e1 = event(b"stream", 0, None);
        let e2 = event(b"stream", 1, Some(hash0));

        store.append_event(e1).unwrap();
        store.append_event(e2.clone()).unwrap();

        let reopened = BlockStreamStore::open(device).unwrap();
        let head = reopened.get_head(&b"stream"[..]).unwrap();
        assert_eq!(head.0, 1);

        let got = reopened.get_event(b"stream", 1).unwrap().unwrap();
        assert_eq!(got.seq, e2.seq);
        assert_eq!(got.stream_id, e2.stream_id);
    }

    #[test]
    fn block_stream_store_stores_non_contiguous_seq_without_authoring_state() {
        let device = InMemoryBlockDevice::new(32, 64);
        let mut store = BlockStreamStore::open(device).unwrap();
        let e0 = event(b"stream", 0, None);
        let e2 = event(b"stream", 2, None);
        store.append_event(e0).unwrap();
        store.append_event(e2).unwrap();
        assert_eq!(store.get_head(b"stream").unwrap().0, 2);
    }

    #[test]
    fn block_stream_store_scan_matches_head() {
        let device = InMemoryBlockDevice::new(32, 64);
        let mut store = BlockStreamStore::open(device).unwrap();

        let e0 = event(b"s", 0, None);
        let e1 = event(b"t", 0, None);
        store.append_event(e0).unwrap();
        store.append_event(e1).unwrap();

        let all = store.scan().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].event.stream_id, b"s");
        assert_eq!(all[1].event.stream_id, b"t");
    }

    #[test]
    fn block_stream_store_does_not_validate_prev_hash() {
        let device = InMemoryBlockDevice::new(32, 64);
        let mut store = BlockStreamStore::open(device).unwrap();

        let e0 = event(b"stream", 0, None);
        let e1 = event(b"stream", 1, Some(vec![0xAA; 32]));
        store.append_event(e0).unwrap();
        store.append_event(e1).unwrap();
        assert_eq!(store.get_head(b"stream").unwrap().0, 1);
    }
}
