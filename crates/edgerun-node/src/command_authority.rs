//! Shared command authority gate.
//!
//! This module centralizes replay, validation, and local policy checks so each
//! command group can be routed without copying the same authority logic.

use crate::command_dispatch::ControllerSet;
use edgerun_core::collections::{HashMap, HashSet};
use edgerun_core::command::{
    command_hash, validate_command, CommandExecutionContext, CommandValidationContext,
};
use edgerun_core::result::Verdict;
use edgerun_core::util::now_unix_millis_i64;
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::stream::CommandEnvelope;
use edgerun_storage::NodeStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandGateDecision {
    Accept,
    CommitDuplicate,
    Reject(String),
}

pub fn command_authority_gate(
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
) -> CommandGateDecision {
    let computed_hash = command_hash(command);
    let cmd_hash_hex = edgerun_core::util::bytes_to_hex(&computed_hash.value);
    if let Some(ref target) = command.target_node {
        let target_hex = edgerun_core::util::bytes_to_hex(&target.node_id);
        if let Ok(Some((_cmd_id, _event_seq))) = store.get_replay_entry(&target_hex, &cmd_hash_hex)
        {
            return CommandGateDecision::CommitDuplicate;
        }
    }

    if replay_cache.contains_key(&computed_hash.value) {
        return CommandGateDecision::CommitDuplicate;
    }

    let validation = validate_command_with_context(
        command,
        replay_cache,
        revoked_delegations,
        trusted_root_ids,
        local_assurance_class,
        exec_ctx,
        signer,
    );

    match validation.verdict {
        Verdict::Reject => {
            let reason = validation
                .reason_code
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "rejected".into());
            return CommandGateDecision::Reject(reason);
        }
        Verdict::Defer => return CommandGateDecision::Reject("deferred".into()),
        Verdict::Duplicate => return CommandGateDecision::CommitDuplicate,
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
    let policy = edgerun_core::command::validate_command_policy(&policy_ctx);
    if policy.verdict == Verdict::Reject {
        let reason = policy
            .reason_code
            .map(|r| r.as_str().to_string())
            .unwrap_or_else(|| "policy_denied".into());
        return CommandGateDecision::Reject(reason);
    }

    CommandGateDecision::Accept
}

fn validate_command_with_context(
    command: &CommandEnvelope,
    replay_cache: &mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
    revoked_delegations: &HashSet<Vec<u8>>,
    trusted_root_ids: &[Vec<u8>],
    local_assurance_class: i32,
    exec_ctx: &CommandExecutionContext,
    signer: &dyn MeshSigner,
) -> edgerun_core::result::ValidationResult {
    let now_ms = now_unix_millis_i64();
    let local_node_id = signer.node_id().0;
    let delegation_use_counts: HashMap<Vec<u8>, u64> = HashMap::new();
    let delegation_rate_events_ms: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();
    let location_classes: Vec<&str> = exec_ctx
        .location_classes
        .iter()
        .map(String::as_str)
        .collect();

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
}
