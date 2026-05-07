//! Direct signed-stream persistence helpers.

use crate::prelude::v1::*;
use edgerun_protocols::core_protocol::protocol::EventEnvelope;
use edgerun_protocols::sign::ProtocolSigner;
use edgerun_stream::{StreamId, StreamWriter};

use crate::core::{AppendReceipt, EventLog};
use crate::error::StorageError;

/// A stream writer that persists every signed event to an `EventLog`.
///
/// `StreamWriter` owns the protocol invariants: single writer, contiguous seq,
/// `prev_event_hash`, and signatures. This type adds durable append as part of
/// the same operation so callers cannot accidentally create signed events that
/// are never written to the authoritative log.
pub struct DurableStreamWriter<L, S> {
    writer: StreamWriter<S>,
    event_log: L,
    last_receipt: AppendReceipt,
}

impl<L: EventLog, S: ProtocolSigner> DurableStreamWriter<L, S> {
    /// Create a stream, sign the genesis event, and persist it immediately.
    pub fn new(
        stream_id: StreamId,
        signer: S,
        recorded_at_ms: i64,
        mut event_log: L,
    ) -> Result<Self, StorageError> {
        let writer = StreamWriter::new(stream_id, signer, recorded_at_ms)?;
        let genesis = writer.head().ok_or_else(|| {
            StorageError::Stream("stream writer did not produce a genesis event".into())
        })?;
        let last_receipt = event_log.append_event(genesis)?;
        event_log.sync()?;

        Ok(Self {
            writer,
            event_log,
            last_receipt,
        })
    }

    /// Append, sign, and persist an event in one operation.
    pub fn append(
        &mut self,
        event_type: i32,
        event_version: u32,
        recorded_at_ms: i64,
    ) -> Result<AppendReceipt, StorageError> {
        let event = self
            .writer
            .append(event_type, event_version, recorded_at_ms)?;
        let receipt = self.event_log.append_event(&event)?;
        self.event_log.sync()?;
        self.last_receipt = receipt.clone();
        Ok(receipt)
    }

    /// Returns the current signed stream head.
    #[must_use]
    pub fn head(&self) -> Option<&EventEnvelope> {
        self.writer.head()
    }

    /// Returns the in-memory events produced by the stream writer.
    #[must_use]
    pub fn events(&self) -> &[EventEnvelope] {
        self.writer.events()
    }

    /// Returns the last durable append receipt.
    #[must_use]
    pub fn last_receipt(&self) -> &AppendReceipt {
        &self.last_receipt
    }

    /// Returns the stream ID.
    #[must_use]
    pub fn stream_id(&self) -> &[u8] {
        self.writer.stream_id()
    }

    /// Returns the writer identity.
    #[must_use]
    pub fn writer(&self) -> &StreamId {
        self.writer.stream_id()
    }

    /// Borrow the underlying event log.
    #[must_use]
    pub fn event_log(&self) -> &L {
        &self.event_log
    }

    /// Mutably borrow the underlying event log.
    pub fn event_log_mut(&mut self) -> &mut L {
        &mut self.event_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::canonical_event_hash;
    use crate::mem::MemEventLog;
    use crate::test_support::TestSigner;

    #[test]
    fn new_persists_signed_genesis() {
        let signer = TestSigner::new();
        let stream_id = signer.node_id();
        let writer =
            DurableStreamWriter::new(stream_id, signer, 1_000, MemEventLog::new()).unwrap();

        let scanned = writer.event_log().scan().unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].event.seq, 0);
        assert!(scanned[0].event.signature.is_some());
        assert_eq!(
            writer.last_receipt().event_hash,
            canonical_event_hash(&scanned[0].event).value
        );
    }

    #[test]
    fn append_persists_contiguous_signed_events() {
        let signer = TestSigner::new();
        let stream_id = signer.node_id();
        let mut writer =
            DurableStreamWriter::new(stream_id, signer, 1_000, MemEventLog::new()).unwrap();

        let receipt = writer.append(100, 1, 1_001).unwrap();
        let scanned = writer.event_log().scan().unwrap();

        assert_eq!(receipt.seq, 1);
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[1].event.seq, 1);
        assert!(scanned[1].event.prev_event_hash.is_some());
        assert!(scanned[1].event.signature.is_some());
        assert_eq!(
            receipt.event_hash,
            canonical_event_hash(&scanned[1].event).value
        );
    }
}
