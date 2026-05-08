use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct SourceBlob {
    pub path: String,
    pub source: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub files: Vec<SourceBlob>,
    pub total_bytes: u64,
}

impl SourceSnapshot {
    pub fn encode_rkyv(&self) -> Result<Vec<u8>, rkyv::rancor::Error> {
        rkyv::to_bytes::<rkyv::rancor::Error>(self).map(|bytes| bytes.to_vec())
    }

    pub fn decode_rkyv(bytes: &[u8]) -> Result<Self, rkyv::rancor::Error> {
        let archived = rkyv::access::<ArchivedSourceSnapshot, rkyv::rancor::Error>(bytes)?;
        rkyv::deserialize::<SourceSnapshot, rkyv::rancor::Error>(archived)
    }
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct LocalConnection {
    pub network_transport: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub state: String,
}

#[derive(Clone, PartialEq, Debug, Archive, Serialize, Deserialize)]
pub struct ConnectionsEnvelope {
    pub connections: Vec<LocalConnection>,
    pub connection_count: u32,
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
