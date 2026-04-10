use edgerun_hardware_signing::NodeID;

/// Request sent from TCP connection handlers to the store task.
pub enum StoreRequest {
    Command {
        /// The raw message bytes (for dedup hashing before decode).
        raw_bytes: Vec<u8>,
        command: edgerun_proto::edgerun::v0::stream::CommandEnvelope,
        /// Peer identity for allowlist check.
        peer_id: Option<Vec<u8>>,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    Query {
        /// The raw message bytes (for dedup hashing before decode).
        raw_bytes: Vec<u8>,
        query: edgerun_proto::edgerun::v0::access::QueryRequest,
        /// Peer identity for allowlist check.
        peer_id: Option<Vec<u8>>,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    /// Produce a snapshot of current stream heads.
    ProduceSnapshot {
        view_type: String,
        completeness: i32,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    /// Fetch a local object by ObjectRef and return its content.
    FetchObject {
        object_ref: edgerun_proto::edgerun::v0::common::ObjectRef,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    /// Send a command to a remote peer over TCP and record CommandSent event.
    SendCommand {
        peer_addr: String,
        command: edgerun_proto::edgerun::v0::stream::CommandEnvelope,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
}

/// Response from the store task back to the TCP handler.
pub enum StoreResponse {
    /// Command was processed successfully — payload is the response.
    Ok(Vec<u8>),
    /// Ingress screening rejected the message.
    Rejected(crate::ingress::IngressResult),
}

/// Reply from the store task back to the TCP handler.
/// Contains the response bytes and the original sender's NodeID.
pub struct MeshReply {
    pub source: NodeID,
    pub response_bytes: Vec<u8>,
}

/// An outbound command to be sent to a remote peer.
pub struct OutboundCommand {
    pub peer_addr: String,
    pub command: edgerun_proto::edgerun::v0::stream::CommandEnvelope,
    pub reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
}

/// A request sent to the store task from the mesh loop (blocking thread).
/// Uses std::sync::mpsc since both sender (mesh loop) and receiver (store task)
/// are on blocking threads.
pub struct MeshCommandRequest {
    pub command: edgerun_proto::edgerun::v0::stream::CommandEnvelope,
    pub raw_bytes: Vec<u8>,
    pub source: NodeID,
    pub reply_tx: edgerun_rt::oneshot::Sender<MeshReply>,
}
