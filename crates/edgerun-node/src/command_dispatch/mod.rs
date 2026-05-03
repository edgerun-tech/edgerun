//! Command dispatch wrapper router.
//!
//! The old implementation remains in `command_dispatch_legacy`. This module
//! becomes the stable public command-dispatch surface and gradually routes
//! command groups to extracted handlers.

pub use crate::command_dispatch_legacy::{
    create_node_genesis_payload, project_config, project_config_from_base,
    project_controller_set, CommandDispatchResult, ControllerSet,
};
pub(crate) use crate::command_dispatch_legacy::{
    append_signed_event, sign_and_append_event, sign_and_append_event_blocking,
    sign_event_envelope,
};

use crate::command_dispatch_event::{append_command_result_event, CommandResultEventWrite};
use crate::command_dispatch_server_resource::dispatch_server_resource_command_result;
use edgerun_core::collections::{HashMap, HashSet};
use edgerun_core::command::{
    command_hash, validate_command, CommandExecutionContext, CommandValidationContext,
};
use edgerun_core::result::Verdict;
use edgerun_core::util::now_unix_millis_i64;
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, EventType};
use edgerun_storage::NodeStore;

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
    exec_ctx: &CommandExecutionContext,
) -> CommandDispatchResult {
    if crate::server_resources::is_server_resource_command(command.command_type) {
        return dispatch_server_resource_command_after_validation(
            command,
            store,
            stream_id,
            signer,
            controllers,
            replay_cache,
            revoked_delegations,
            trusted_root_ids,
            local_assurance_class,
            exec_ctx,
        );
    }

    crate::command_dispatch_legacy::dispatch_command(
        command,
        store,
        stream_id,
        signer,
        controllers,
        replay_cache,
        revoked_delegations,
        trusted_root_ids,
        local_assurance_class,
        exec_ctx,
    )
}

fn dispatch_server_resource_command_after_validation(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    replay_cache: &mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
    revoked_delegations: &HashSet<Vec<u8>>,
    trusted_root_ids: &[Vec<u8>],
    local_assurance_class: i32,
    exec_ctx: &CommandExecutionContext,
) -> CommandDispatchResult {
    let computed_hash = command_hash(command);
    let cmd_hash_hex = edgerun_core::util::bytes_to_hex(&computed_hash.value);
    if let Some(ref target) = command.target_node {
        let target_hex = edgerun_core::util::bytes_to_hex(&target.node_id);
        if let Ok(Some((_cmd_id, _event_seq))) = store.get_replay_entry(&target_hex, &cmd_hash_hex)
        {
            return append_decision(command, store, stream_id, signer, true, "duplicate_command");
        }
    }

    if replay_cache.contains_key(&computed_hash.value) {
        return append_decision(command, store, stream_id, signer, true, "duplicate_command");
    }

    let now_ms = now_unix_millis_i64();
    let local_node_id = signer.node_id().0;
    let delegation_use_counts: HashMap<Vec<u8>, u64> = HashMap::new();
    let delegation_rate_events_ms: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();
    let location_classes: Vec<&str> = exec_ctx
        .location_classes
        .iter()
        .map(String::as_str)
        .collect();

    let validation_result = {
        let ctx = CommandValidationContext {
            local_node_id: &local_node_id,
            replay_cache,
            revoked_delegation_ids: revoked_delegations,
            delegation_use_counts: &delegation_use_counts,
            delegation_rate_events_ms: &delegation_rate_events_ms,
            now_ms,
            trusted_root_ids,
            local_assurance_class,
            accepted_assurance_claims: &exec_ctx.accepted_assurance_claims,
            has_local_session: exec_ctx.has_local_session,
            has_user_presence: exec_ctx.has_user_presence,
            transport_class: exec_ctx.transport_class.as_deref().and_then(|s| match s {
                "lan" => Some(1),
                "mesh" => Some(2),
                "internet" => Some(3),
                _ => None,
            }),
            location_classes: &location_classes,
            target_stream_id: exec_ctx.target_stream_id.as_deref(),
            target_view_type: exec_ctx.target_view_type.as_deref(),
            target_domain: exec_ctx.target_domain.as_deref(),
            execution_class: exec_ctx.execution_class.as_deref().and_then(|s| match s {
                "wasm" => Some(1),
                "native" => Some(2),
                "container" => Some(3),
                _ => None,
            }),
            storage_class: exec_ctx.storage_class.as_deref().and_then(|s| match s {
                "file" => Some(1),
                "memory" => Some(2),
                "object" => Some(3),
                _ => None,
            }),
        };
        validate_command(command, &ctx)
    };

    match validation_result.verdict {
        Verdict::Reject => {
            let reason = validation_result
                .reason_code
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "rejected".into());
            return append_decision(command, store, stream_id, signer, false, &reason);
        }
        Verdict::Defer => {
            return append_decision(command, store, stream_id, signer, false, "deferred");
        }
        Verdict::Duplicate => {
            return append_decision(command, store, stream_id, signer, true, "duplicate_command");
        }
        Verdict::Accept => {}
    }

    let issuer_id = command
        .issuer
        .as_ref()
        .map(|i| i.identity_id.clone())
        .unwrap_or_default();
    let policy_ctx = edgerun_core::command::CommandPolicyContext {
        issuer_identity_id: &issuer_id,
        command_type: command.command_type,
        has_valid_delegation: !command.delegation_chain.is_empty(),
        controller_ids: &controllers.to_vec(),
        allowed_command_types: &[],
    };
    let policy_result = edgerun_core::command::validate_command_policy(&policy_ctx);
    if policy_result.verdict == Verdict::Reject {
        let reason = policy_result
            .reason_code
            .map(|r| r.as_str().to_string())
            .unwrap_or_else(|| "policy_denied".into());
        return append_decision(command, store, stream_id, signer, false, &reason);
    }

    dispatch_server_resource_command_result(command, store, stream_id, signer)
}

fn append_decision(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    committed: bool,
    reason_code: &str,
) -> CommandDispatchResult {
    match append_command_result_event(
        command,
        store,
        stream_id,
        signer,
        committed,
        reason_code,
        None,
    ) {
        Ok(write) => result_from_event_write(write, committed, reason_code.to_string()),
        Err(reason) => CommandDispatchResult {
            event_type: EventType::CommandRejected,
            decision: CommandDecision::Rejected as i32,
            reason_code: reason,
            response_bytes: Vec::new(),
        },
    }
}

fn result_from_event_write(
    write: CommandResultEventWrite,
    committed: bool,
    reason_code: String,
) -> CommandDispatchResult {
    CommandDispatchResult {
        event_type: write.event_type,
        decision: if committed {
            CommandDecision::Committed as i32
        } else {
            CommandDecision::Rejected as i32
        },
        reason_code,
        response_bytes: write.response_bytes,
    }
}
