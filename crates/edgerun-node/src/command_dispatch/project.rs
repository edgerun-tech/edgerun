use edgerun_log;
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use edgerun_core::result::Verdict;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use edgerun_proto::edgerun::v0::common::{CommandRef, DelegationRef};
use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use prost::Message;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};


// ---------------------------------------------------------------------------
// Controller projection from event log
// ---------------------------------------------------------------------------

/// Projects the current controller set from the genesis event + all command events.
///
/// Walks the event log from seq 0, processing:
/// - NodeGenesis: sets initial controllers
/// - CommandCommitted with ADD_CONTROLLER: adds controller
/// Replays controller changes from the persistent change log to rebuild the ControllerSet.
///
/// Delegates to `NodeStore::project_controller_set()` which reads from the
/// FileIndex's `controller_changes.bin` — an append-only log of all controller
/// mutations recorded during command processing.
///
/// This ensures controller state survives node restarts.
pub fn project_controller_set(
    store: &NodeStore,
    _stream_id: &[u8],
    initial_controllers: Vec<Vec<u8>>,
) -> ControllerSet {
    let head_seq = match store.get_head(_stream_id) {
        Ok(Some((seq, _))) => seq,
        _ => return ControllerSet::new(initial_controllers),
    };

    match store.project_controller_set(initial_controllers, head_seq) {
        Ok(set) => ControllerSet::new(set.to_vec()),
        Err(e) => {
