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

            ControllerSet::new(Vec::new())
        }
    }
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Processes a command through full validation and type-specific dispatch.
///
/// `workload_policy` and `rate_limiter` are projected from the immutable
/// event log by the caller — they are NOT loaded from mutable files here.
///
/// `running_workloads` is a shared registry for tracking active containers
/// so they can be preempted.
///
/// `local_assurance_class` is the node's assurance capability
/// (ASSURANCE_CLASS_SOFTWARE=1, HARDWARE_BACKED=2, ATTESTED_RUNTIME=3).
///
/// Returns the response bytes to send back to the caller.
pub fn dispatch_command(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    replay_cache: &mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
    revoked_delegations: &HashSet<Vec<u8>>,
    trusted_root_ids: &[Vec<u8>],
    local_assurance_class: i32,
    capacity_tracker: &std::sync::Arc<crate::capacity::ResourceTracker>,
    workload_policy: &super::workload_policy::WorkloadPolicy,
    rate_limiter: &super::workload_policy::RateLimiter,
    running_workloads: &std::sync::Arc<super::running_workloads::RunningWorkloads>,
) -> CommandDispatchResult {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_millis() as i64;

    let local_node_id = signer.node_id().0;

    // Build validation context for full validate_command
    let ctx = CommandValidationContext {
        local_node_id: &local_node_id,
        replay_cache,
        revoked_delegation_ids: revoked_delegations,
        now_ms,
        trusted_root_ids,
        local_assurance_class,
    };

    // Run full validation (replay, timing, delegation chain, cryptographic signatures)
    // validate_command already verifies both command and delegation chain signatures,
    // so we don't need separate verify_command_signature/verify_delegation_signature calls.
    let validation_result = validate_command(command, &ctx);

    // If validation rejected or deferred, record and return
    match validation_result.verdict {
        Verdict::Reject => {
            let reason = validation_result.reason_code.map(|r| r.as_str().to_string()).unwrap_or_else(|| "rejected".into());
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, &reason, Vec::new(), None);
        }
        Verdict::Defer => {
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "deferred", Vec::new(), None);
        }
        Verdict::Duplicate => {
            // Already processed — return cached result
            // For now, just re-process (in production we'd cache results)
        }
        Verdict::Accept => {}
    }

    // Check if issuer is a controller (or has valid delegation)
    let issuer_id = command.issuer.as_ref().map(|i| i.identity_id.clone()).unwrap_or_default();
    if !controllers.contains(&issuer_id) && command.delegation_chain.is_empty() {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "not_a_controller", Vec::new(), None);
    }

    // Dispatch by command type
    let command_type = command.command_type;
    match command_type {
        x if x == CommandType::AddController as i32 => {
            dispatch_add_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RemoveController as i32 => {
            dispatch_remove_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::TransferControl as i32 => {
            dispatch_transfer_control(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::PublishSnapshot as i32 => {
            // Snapshot publishing is handled via ProduceSnapshot request
            record_and_respond(command, store, stream_id, signer, controllers,
                false, "use_produce_snapshot_request", Vec::new(), None)
        }
        x if x == CommandType::FetchObject as i32 => {
            // Object fetching is handled via FetchObject request
            record_and_respond(command, store, stream_id, signer, controllers,
                false, "use_fetch_object_request", Vec::new(), None)
        }
        x if x == CommandType::Query as i32 => {
            // Queries are handled via Query request
            record_and_respond(command, store, stream_id, signer, controllers,
                true, "", Vec::new(), None)
        }
        #[cfg(feature = "oci")]
        x if x == CommandType::ExecuteWorkload as i32 => {
            // Execute workload with full accounting + capacity check
            dispatch_execute_workload(command, store, stream_id, signer, controllers, capacity_tracker, workload_policy, rate_limiter, running_workloads)
        }
        #[cfg(not(feature = "oci"))]
        x if x == CommandType::ExecuteWorkload as i32 => {
            record_and_respond(command, store, stream_id, signer, controllers,
                false, "oci_feature_not_enabled", Vec::new(), None)
        }
        x if x == CommandType::TerminateWorkload as i32 => {
            // Preempt/terminate a running workload
            dispatch_terminate_workload(command, store, stream_id, signer, controllers, capacity_tracker, running_workloads)
        }
