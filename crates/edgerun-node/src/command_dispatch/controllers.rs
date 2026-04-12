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

            // Custom commands: try to decode as DelegationRecord or RevocationRecord
            dispatch_custom_command(command, store, stream_id, signer, controllers)
        }
        _ => {
            // Unknown or custom command — accept but don't execute
            record_and_respond(command, store, stream_id, signer, controllers,
                true, "", Vec::new(), None)
        }
    }
}

// ---------------------------------------------------------------------------
// Command type handlers
// ---------------------------------------------------------------------------

fn dispatch_add_controller(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    // Extract the new controller identity from the command's payload
    let new_controller_id = extract_identity_from_command(command);
    if new_controller_id.is_empty() {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_controller_identity", Vec::new(), None);
    }

    // Add to controller set
    controllers.add(new_controller_id.clone());

    edgerun_log::info!("controller added");

    let response = format!("controller added: {}", edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id), "added")))
}

fn dispatch_remove_controller(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let target_id = extract_identity_from_command(command);
    if target_id.is_empty() {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_controller_identity", Vec::new(), None);
    }

    if !controllers.remove(&target_id) {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "controller_not_found", Vec::new(), None);
    }

    edgerun_log::info!("controller removed");

    let response = format!("controller removed: {}", edgerun_core::util::bytes_to_hex_prefixed(&target_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&edgerun_core::util::bytes_to_hex_prefixed(&target_id), "removed")))
}

fn dispatch_transfer_control(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    // Extract new controller identity
    let new_controller_id = extract_identity_from_command(command);
    if new_controller_id.is_empty() {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_target_identity", Vec::new(), None);
    }

    // Safe transfer: add new controller first (removing old ones is manual)
    controllers.add(new_controller_id.clone());

    edgerun_log::info!("control transfer initiated — new controller added, old controllers remain until explicitly removed");

    let response = format!("control transferred to: {}", edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id), "transferred")))
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// ExecuteWorkload — compute marketplace with full accounting
// ---------------------------------------------------------------------------

/// Dispatches an ExecuteWorkload command: parse image spec, pull via OCI
/// registry, run the container via the OCI runtime, and record full
/// resource accounting — all billed in reference-core-microseconds.
///
/// The container is started non-blocking and registered in `running_workloads`
/// so it can be preempted. A background thread awaits its completion and
/// records accounting.
#[cfg(feature = "oci")]
fn dispatch_execute_workload(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    capacity_tracker: &std::sync::Arc<crate::capacity::ResourceTracker>,
    workload_policy: &super::workload_policy::WorkloadPolicy,
    rate_limiter: &super::workload_policy::RateLimiter,
    running_workloads: &std::sync::Arc<super::running_workloads::RunningWorkloads>,
) -> CommandDispatchResult {
    use super::metering::{WorkMeter, compute_work_id};
    use edgerun_core::accounting::{WorkloadClass, WorkPriority, WorkStatus};
    use std::sync::Arc;

