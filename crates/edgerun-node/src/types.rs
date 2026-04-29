use edgerun_hardware_signing::NodeID;

/// Request sent from TCP connection handlers to the store task.
pub enum StoreRequest {
    Command {
        /// The raw message bytes (for dedup hashing before decode).
        raw_bytes: Vec<u8>,
        command: edgerun_proto::edgerun::v0::stream::CommandEnvelope,
        /// Peer identity for allowlist check.
        peer_id: Option<Vec<u8>>,
        /// Reply channel. `None` for fire-and-forget (e.g., mesh commands).
        reply_tx: Option<edgerun_rt::oneshot::Sender<StoreResponse>>,
    },
    Query {
        /// The raw message bytes (for dedup hashing before decode).
        raw_bytes: Vec<u8>,
        query: edgerun_proto::edgerun::v0::access::QueryRequest,
        /// Peer identity for allowlist check.
        peer_id: Option<Vec<u8>>,
        /// Reply channel. `None` for fire-and-forget (e.g., mesh commands).
        reply_tx: Option<edgerun_rt::oneshot::Sender<StoreResponse>>,
    },
    /// Build a signed advisory aggregate from local query results plus remote
    /// signed `QueryResultFragment` bytes.
    FederatedQuery {
        query: edgerun_proto::edgerun::v0::access::QueryRequest,
        remote_fragments: Vec<Vec<u8>>,
        trusted_responders: Vec<Vec<u8>>,
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
    /// Dequeue one pending fetch entry from the queue (for remote peer querying).
    FetchDequeue {
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    /// Mark a fetch entry as done after successful remote retrieval.
    FetchMarkDone {
        fetch_id: i64,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    /// Re-enqueue a fetch entry with lower priority (peer query failed).
    FetchRequeue {
        target_type: String,
        target_id: String,
        priority: i64,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    /// Periodic maintenance tick — triggers WAL checkpoint, integrity check, etc.
    /// Sent by a timer thread to ensure maintenance runs even during quiet periods.
    MaintenanceTick,
    /// Update a peer's connectivity status.
    PeerStatusUpdate { node_id_hex: String, status: String },
    /// Look up a peer's address by node_id_hex.
    PeerLookup {
        node_id_hex: String,
        reply_tx: edgerun_rt::oneshot::Sender<StoreResponse>,
    },
    /// Signal the store task to shut down gracefully, terminating all workloads.
    Shutdown,
    /// Reload configuration from the YAML file — updates allowed_peers,
    /// bootstrap_peers, controllers, trust_nodes, and workload policy.
    ConfigReload {
        config: crate::config::NodeConfig,
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
/// Uses `edgerun_rt::mpsc` (async-compatible channel) since the sender
/// is on the mesh blocking thread and the receiver is on the store blocking thread.
pub struct MeshCommandRequest {
    pub command: edgerun_proto::edgerun::v0::stream::CommandEnvelope,
    pub raw_bytes: Vec<u8>,
    pub source: NodeID,
    pub reply_tx: edgerun_rt::oneshot::Sender<MeshReply>,
}
