pub use crate::graph_types::{
    ArchivedConnectionsEnvelope, ArchivedSourceSnapshot, ConnectionsEnvelope, SourceSnapshot,
};
pub use crate::graph_types::LocalConnection;

impl SourceSnapshot {
    pub fn encode_rkyv(&self) -> Result<Vec<u8>, rkyv::rancor::Error> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self).map(|bytes| bytes.to_vec())
    }

    pub fn decode_rkyv(bytes: &[u8]) -> Result<Self, rkyv::rancor::Error> {
        let archived = rkyv::access::<ArchivedSourceSnapshot, rkyv::rancor::Error>(bytes)?;
        rkyv::deserialize::<SourceSnapshot, rkyv::rancor::Error>(archived)
    }
}

impl ConnectionsEnvelope {
    pub fn encode_rkyv(&self) -> Result<Vec<u8>, rkyv::rancor::Error> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self).map(|bytes| bytes.to_vec())
    }

    pub fn decode_rkyv(bytes: &[u8]) -> Result<Self, rkyv::rancor::Error> {
        let archived = rkyv::access::<ArchivedConnectionsEnvelope, rkyv::rancor::Error>(bytes)?;
        rkyv::deserialize::<ConnectionsEnvelope, rkyv::rancor::Error>(archived)
    }
}
