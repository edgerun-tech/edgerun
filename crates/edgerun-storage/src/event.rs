// SPDX-License-Identifier: GPL-2.0-only
use blake3::Hasher;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EventError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid magic: {0}")]
    InvalidMagic(u32),
    #[error("Invalid version: {0}")]
    InvalidVersion(u16),
    #[error("CRC mismatch: expected {expected}, got {actual}")]
    CrcMismatch { expected: u32, actual: u32 },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub const MAGIC: u32 = 0x45564554;
pub const SEG0_MAGIC: u32 = 0x53454730;
pub const VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventFlags {
    pub compressed: bool,
    pub encrypted: bool,
    pub tombstone: bool,
}

impl EventFlags {
    pub fn to_u16(&self) -> u16 {
        (self.compressed as u16) << 0 | (self.encrypted as u16) << 1 | (self.tombstone as u16) << 2
    }

    pub fn from_u16(val: u16) -> Self {
        Self {
            compressed: (val & (1 << 0)) != 0,
            encrypted: (val & (1 << 1)) != 0,
            tombstone: (val & (1 << 2)) != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CrdtType {
    #[default]
    None,
    OrSet,
    LwwRegister,
    PnCounter,
    RgaList,
    Map,
}

impl CrdtType {
    pub fn to_u16(&self) -> u16 {
        match self {
            CrdtType::None => 0,
            CrdtType::OrSet => 1,
            CrdtType::LwwRegister => 2,
            CrdtType::PnCounter => 3,
            CrdtType::RgaList => 4,
            CrdtType::Map => 5,
        }
    }

    pub fn from_u16(val: u16) -> Self {
        match val {
            0 => CrdtType::None,
            1 => CrdtType::OrSet,
            2 => CrdtType::LwwRegister,
            3 => CrdtType::PnCounter,
            4 => CrdtType::RgaList,
            5 => CrdtType::Map,
            _ => CrdtType::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HlcTimestamp {
    pub physical: i64,
    pub logical: u32,
}

impl HlcTimestamp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn now() -> Self {
        let physical = Utc::now().timestamp_millis();
        Self {
            physical,
            logical: 0,
        }
    }

    pub fn tick(&mut self) {
        let now = Utc::now().timestamp_millis();
        if now > self.physical {
            self.physical = now;
            self.logical = 0;
        } else {
            self.logical += 1;
        }
    }

    pub fn to_bytes(&self) -> [u8; 12] {
        let mut bytes = [0u8; 12];
        bytes[..8].copy_from_slice(&self.physical.to_le_bytes());
        bytes[8..].copy_from_slice(&self.logical.to_le_bytes());
        bytes
    }

    pub fn from_bytes(data: &[u8; 12]) -> Self {
        let physical = i64::from_le_bytes(data[..8].try_into().unwrap());
        let logical = u32::from_le_bytes(data[8..].try_into().unwrap());
        Self { physical, logical }
    }
}

impl fmt::Display for HlcTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.physical, self.logical)
    }
}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.physical
            .cmp(&other.physical)
            .then_with(|| self.logical.cmp(&other.logical))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId(pub [u8; 16]);

impl StreamId {
    pub fn new() -> Self {
        use rand::Rng;
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Default for StreamId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub [u8; 16]);

impl EventId {
    pub fn new() -> Self {
        use rand::Rng;
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill(&mut bytes);
        Self(bytes)
    }

    pub fn from_counter(counter: u64, actor_id: &[u8; 16]) -> Self {
        let mut bytes = *actor_id;
        bytes[..8].copy_from_slice(&counter.to_le_bytes());
        Self(bytes)
    }

    pub fn counter(&self) -> u64 {
        u64::from_le_bytes(self.0[..8].try_into().unwrap())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub [u8; 16]);

impl ActorId {
    pub fn new() -> Self {
        use rand::Rng;
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub stream_id: StreamId,
    pub event_id: EventId,
    pub hlc_timestamp: HlcTimestamp,
    pub actor_id: ActorId,
    pub prev_hash: Option<[u8; 32]>,
    pub deps: Vec<[u8; 32]>,
    pub schema_id: u32,
    pub payload: Vec<u8>,
    pub flags: EventFlags,
    pub crdt_type: CrdtType,
}

impl Event {
    pub fn new(stream_id: StreamId, actor_id: ActorId, payload: Vec<u8>) -> Self {
        Self {
            stream_id,
            event_id: EventId::new(),
            hlc_timestamp: HlcTimestamp::now(),
            actor_id,
            prev_hash: None,
            deps: Vec::new(),
            schema_id: 0,
            payload,
            flags: EventFlags::default(),
            crdt_type: CrdtType::default(),
        }
    }

    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();

        hasher.update(&MAGIC.to_le_bytes());
        hasher.update(&VERSION.to_le_bytes());
        hasher.update(&self.flags.to_u16().to_le_bytes());
        hasher.update(&self.stream_id.0);
        hasher.update(&self.event_id.0);
        hasher.update(&self.hlc_timestamp.to_bytes());
        hasher.update(&self.actor_id.0);

        if let Some(prev) = self.prev_hash {
            hasher.update(&prev);
        }

        hasher.update(&(self.deps.len() as u32).to_le_bytes());
        for dep in &self.deps {
            hasher.update(dep);
        }

        hasher.update(&self.schema_id.to_le_bytes());
        hasher.update(&self.payload);

        *hasher.finalize().as_bytes()
    }

    pub fn serialize(&self) -> Result<Vec<u8>, EventError> {
        let mut result = Vec::new();

        result.extend_from_slice(&MAGIC.to_le_bytes());
        result.extend_from_slice(&VERSION.to_le_bytes());
        result.extend_from_slice(&self.flags.to_u16().to_le_bytes());

        let payload_len = self.payload.len() as u32;
        result.extend_from_slice(&payload_len.to_le_bytes());

        let event_hash = self.compute_hash();
        result.extend_from_slice(&event_hash);

        if let Some(prev) = self.prev_hash {
            result.extend_from_slice(&prev);
        } else {
            result.extend_from_slice(&[0u8; 32]);
        }

        result.extend_from_slice(&self.actor_id.0);

        let hlc_bytes = self.hlc_timestamp.to_bytes();
        result.extend_from_slice(&hlc_bytes);

        result.extend_from_slice(&(self.deps.len() as u32).to_le_bytes());
        for dep in &self.deps {
            result.extend_from_slice(dep);
        }

        result.extend_from_slice(&self.schema_id.to_le_bytes());

        let header_crc = crc32fast::hash(&result);
        result.extend_from_slice(&header_crc.to_le_bytes());

        result.extend_from_slice(&self.payload);

        let payload_crc = crc32fast::hash(&self.payload);
        result.extend_from_slice(&payload_crc.to_le_bytes());

        Ok(result)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, EventError> {
        if data.len() < 4 + 2 + 2 + 4 + 32 + 32 + 16 + 12 + 4 + 4 + 4 {
            return Err(EventError::Serialization("Data too short".to_string()));
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != MAGIC {
            return Err(EventError::InvalidMagic(magic));
        }

        let version = u16::from_le_bytes([data[4], data[5]]);
        if version != VERSION {
            return Err(EventError::InvalidVersion(version));
        }

        let flags = EventFlags::from_u16(u16::from_le_bytes([data[6], data[7]]));

        let offset = 8;
        let payload_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;

        let _event_hash: [u8; 32] = [
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            data[offset + 11],
            data[offset + 12],
            data[offset + 13],
            data[offset + 14],
            data[offset + 15],
            data[offset + 16],
            data[offset + 17],
            data[offset + 18],
            data[offset + 19],
            data[offset + 20],
            data[offset + 21],
            data[offset + 22],
            data[offset + 23],
            data[offset + 24],
            data[offset + 25],
            data[offset + 26],
            data[offset + 27],
            data[offset + 28],
            data[offset + 29],
            data[offset + 30],
            data[offset + 31],
            data[offset + 32],
            data[offset + 33],
            data[offset + 34],
            data[offset + 35],
        ];

        let mut pos = offset + 36;

        let prev_hash_bytes: [u8; 32] = [
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
            data[pos + 8],
            data[pos + 9],
            data[pos + 10],
            data[pos + 11],
            data[pos + 12],
            data[pos + 13],
            data[pos + 14],
            data[pos + 15],
            data[pos + 16],
            data[pos + 17],
            data[pos + 18],
            data[pos + 19],
            data[pos + 20],
            data[pos + 21],
            data[pos + 22],
            data[pos + 23],
            data[pos + 24],
            data[pos + 25],
            data[pos + 26],
            data[pos + 27],
            data[pos + 28],
            data[pos + 29],
            data[pos + 30],
            data[pos + 31],
        ];
        let prev_hash = if prev_hash_bytes.iter().all(|&b| b == 0) {
            None
        } else {
            Some(prev_hash_bytes)
        };
        pos += 32;

        let actor_id: [u8; 16] = [
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
            data[pos + 8],
            data[pos + 9],
            data[pos + 10],
            data[pos + 11],
            data[pos + 12],
            data[pos + 13],
            data[pos + 14],
            data[pos + 15],
        ];
        pos += 16;

        let hlc_bytes: [u8; 12] = [
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
            data[pos + 8],
            data[pos + 9],
            data[pos + 10],
            data[pos + 11],
        ];
        let hlc_timestamp = HlcTimestamp::from_bytes(&hlc_bytes);
        pos += 12;

        let deps_count =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        let mut deps = Vec::new();
        for _ in 0..deps_count {
            let dep: [u8; 32] = [
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
                data[pos + 4],
                data[pos + 5],
                data[pos + 6],
                data[pos + 7],
                data[pos + 8],
                data[pos + 9],
                data[pos + 10],
                data[pos + 11],
                data[pos + 12],
                data[pos + 13],
                data[pos + 14],
                data[pos + 15],
                data[pos + 16],
                data[pos + 17],
                data[pos + 18],
                data[pos + 19],
                data[pos + 20],
                data[pos + 21],
                data[pos + 22],
                data[pos + 23],
                data[pos + 24],
                data[pos + 25],
                data[pos + 26],
                data[pos + 27],
                data[pos + 28],
                data[pos + 29],
                data[pos + 30],
                data[pos + 31],
            ];
            deps.push(dep);
            pos += 32;
        }

        let schema_id =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        let stored_header_crc =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let computed_header_crc = crc32fast::hash(&data[..pos]);
        if stored_header_crc != computed_header_crc {
            return Err(EventError::CrcMismatch {
                expected: stored_header_crc,
                actual: computed_header_crc,
            });
        }

        pos += 4;

        let payload = data[pos..pos + payload_len].to_vec();
        pos += payload_len;

        let stored_payload_crc =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let computed_payload_crc = crc32fast::hash(&payload);
        if stored_payload_crc != computed_payload_crc {
            return Err(EventError::CrcMismatch {
                expected: stored_payload_crc,
                actual: computed_payload_crc,
            });
        }

        Ok(Self {
            stream_id: StreamId::default(),
            event_id: EventId::default(),
            hlc_timestamp,
            actor_id: ActorId::from_bytes(actor_id),
            prev_hash,
            deps,
            schema_id,
            payload,
            flags,
            crdt_type: CrdtType::default(),
        })
    }

    pub fn with_prev_hash(mut self, prev_hash: [u8; 32]) -> Self {
        self.prev_hash = Some(prev_hash);
        self
    }

    pub fn with_deps(mut self, deps: Vec<[u8; 32]>) -> Self {
        self.deps = deps;
        self
    }

    pub fn with_crdt_type(mut self, crdt_type: CrdtType) -> Self {
        self.crdt_type = crdt_type;
        self
    }

    pub fn with_schema_id(mut self, schema_id: u32) -> Self {
        self.schema_id = schema_id;
        self
    }

    pub fn with_tombstone(mut self) -> Self {
        self.flags.tombstone = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialize_deserialize() {
        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        let event = Event::new(stream_id, actor_id, b"test payload".to_vec());

        let serialized = event.serialize().unwrap();
        let deserialized = Event::deserialize(&serialized).unwrap();

        assert_eq!(event.actor_id.0, deserialized.actor_id.0);
        assert_eq!(event.payload, deserialized.payload);
    }

    #[test]
    fn test_event_hash() {
        let actor_id = ActorId::from_bytes(*b"1234567890123456");
        let stream_id = StreamId::new();

        let event = Event::new(
            stream_id.clone(),
            actor_id.clone(),
            b"test payload".to_vec(),
        );
        let hash1 = event.compute_hash();

        let event2 = Event::new(stream_id, actor_id, b"test payload".to_vec());
        let hash2 = event2.compute_hash();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hlc_timestamp() {
        let ts = HlcTimestamp::now();
        assert!(ts.physical > 0);

        let bytes = ts.to_bytes();
        let ts2 = HlcTimestamp::from_bytes(&bytes);

        assert_eq!(ts.physical, ts2.physical);
        assert_eq!(ts.logical, ts2.logical);
    }

    #[test]
    fn test_hlc_timestamp_ordering() {
        let ts1 = HlcTimestamp {
            physical: 1000,
            logical: 0,
        };
        let ts2 = HlcTimestamp {
            physical: 1001,
            logical: 0,
        };

        assert!(ts1 < ts2);
    }

    #[test]
    fn test_event_flags() {
        let mut flags = EventFlags::default();
        assert!(!flags.compressed);

        flags.compressed = true;
        flags.encrypted = true;
        flags.tombstone = true;

        let val = flags.to_u16();
        let flags2 = EventFlags::from_u16(val);

        assert!(flags2.compressed);
        assert!(flags2.encrypted);
        assert!(flags2.tombstone);
    }

    #[test]
    fn test_crdt_type() {
        assert_eq!(CrdtType::OrSet.to_u16(), 1);
        assert_eq!(CrdtType::LwwRegister.to_u16(), 2);
        assert_eq!(CrdtType::PnCounter.to_u16(), 3);

        assert_eq!(CrdtType::from_u16(1), CrdtType::OrSet);
        assert_eq!(CrdtType::from_u16(99), CrdtType::None);
    }

    #[test]
    fn test_stream_id() {
        let id = StreamId::new();
        let bytes = id.as_bytes();
        assert_eq!(bytes.len(), 16);

        let id2 = StreamId::from_bytes(*bytes);
        assert_eq!(id.0, id2.0);
    }

    #[test]
    fn test_event_id() {
        let actor_id = ActorId::new();
        let id = EventId::from_counter(42, &actor_id.0);
        assert_eq!(id.counter(), 42);
    }

    #[test]
    fn test_event_with_options() {
        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        let event = Event::new(stream_id, actor_id, b"test".to_vec())
            .with_prev_hash([1u8; 32])
            .with_deps(vec![[2u8; 32]])
            .with_crdt_type(CrdtType::LwwRegister)
            .with_schema_id(5)
            .with_tombstone();

        assert!(event.prev_hash.is_some());
        assert_eq!(event.deps.len(), 1);
        assert_eq!(event.crdt_type, CrdtType::LwwRegister);
        assert_eq!(event.schema_id, 5);
        assert!(event.flags.tombstone);
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = vec![0u8; 200];
        data[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());

        let result = Event::deserialize(&data);
        assert!(matches!(result, Err(EventError::InvalidMagic(_))));
    }

    #[test]
    fn test_invalid_version() {
        let mut data = vec![0u8; 200];
        data[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        data[4..6].copy_from_slice(&999u16.to_le_bytes());

        let result = Event::deserialize(&data);
        assert!(matches!(result, Err(EventError::InvalidVersion(_))));
    }

    #[test]
    fn test_crc_mismatch() {
        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        let event = Event::new(stream_id, actor_id, b"test payload".to_vec());
        let mut serialized = event.serialize().unwrap();

        serialized[70] ^= 0xFF;

        let result = Event::deserialize(&serialized);
        assert!(matches!(result, Err(EventError::CrcMismatch { .. })));
    }

    #[test]
    fn test_event_deserialize_too_short() {
        let data = vec![0u8; 10];
        let result = Event::deserialize(&data);
        assert!(matches!(result, Err(EventError::Serialization(_))));
    }

    #[test]
    fn test_event_with_prev_hash() {
        let actor_id = ActorId::new();
        let stream_id = StreamId::new();
        let prev_hash: [u8; 32] = [1u8; 32];

        let event = Event::new(stream_id, actor_id, b"test".to_vec()).with_prev_hash(prev_hash);

        assert!(event.prev_hash.is_some());
        assert_eq!(event.prev_hash.unwrap(), prev_hash);
    }

    #[test]
    fn test_event_with_deps() {
        let actor_id = ActorId::new();
        let stream_id = StreamId::new();
        let deps = vec![[1u8; 32], [2u8; 32]];

        let event = Event::new(stream_id, actor_id, b"test".to_vec()).with_deps(deps.clone());

        assert_eq!(event.deps.len(), 2);
    }

    #[test]
    fn test_event_flags_all() {
        let mut flags = EventFlags::default();

        flags.compressed = true;
        assert!(flags.compressed);

        flags.encrypted = true;
        assert!(flags.encrypted);

        flags.tombstone = true;
        assert!(flags.tombstone);
    }

    #[test]
    fn test_crdt_type_values() {
        assert_eq!(CrdtType::None.to_u16(), 0);
        assert_eq!(CrdtType::OrSet.to_u16(), 1);
        assert_eq!(CrdtType::LwwRegister.to_u16(), 2);
        assert_eq!(CrdtType::PnCounter.to_u16(), 3);
        assert_eq!(CrdtType::RgaList.to_u16(), 4);
        assert_eq!(CrdtType::Map.to_u16(), 5);

        assert_eq!(CrdtType::from_u16(0), CrdtType::None);
        assert_eq!(CrdtType::from_u16(6), CrdtType::None);
    }

    #[test]
    fn test_hlc_timestamp_tick() {
        let mut ts = HlcTimestamp::now();
        let physical = ts.physical;

        ts.tick();

        assert!(ts.physical >= physical);
    }

    #[test]
    fn test_actor_id_new() {
        let actor = ActorId::new();
        assert_eq!(actor.0.len(), 16);
    }

    #[test]
    fn test_stream_id_default() {
        let id: StreamId = Default::default();
        assert_eq!(id.0.len(), 16);
    }

    #[test]
    fn test_event_id_default() {
        let id: EventId = Default::default();
        assert_eq!(id.0.len(), 16);
    }

    #[test]
    fn test_event_id_counter() {
        let actor_id = ActorId::new();
        let id = EventId::from_counter(100, &actor_id.0);
        assert_eq!(id.counter(), 100);
    }
}
