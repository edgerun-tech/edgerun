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

/// Check if a command is a duplicate using both persistent and in-memory replay caches.
///
/// Returns `Some(CommandGateDecision::CommitDuplicate)` if the command was already processed,
/// or `None` if it's a new command.
pub fn check_replay_cache(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    replay_cache: &HashMap<Vec<u8>, (Vec<u8>, i64)>,
) -> Option<CommandGateDecision> {
    let computed_hash = command_hash(command);
    let cmd_hash_hex = edgerun_core::util::bytes_to_hex(&computed_hash.value);
    if let Some(ref target) = command.target_node {
        let target_hex = edgerun_core::util::bytes_to_hex(&target.node_id);
        if let Ok(Some((_cmd_id, _event_seq))) = store.get_replay_entry(&target_hex, &cmd_hash_hex)
        {
            return Some(CommandGateDecision::CommitDuplicate);
        }
    }

    if replay_cache.contains_key(&computed_hash.value) {
        return Some(CommandGateDecision::CommitDuplicate);
    }

    None
}

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
    // Check replay cache first
    if let Some(decision) = check_replay_cache(command, store, replay_cache) {
        return decision;
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

/// Build a CommandValidationContext from the execution context and node-local state.
///
/// This eliminates duplicated context-building logic between command_dispatch.rs
/// and command_authority.rs.
pub fn build_validation_context<'a>(
    signer: &'a dyn MeshSigner,
    replay_cache: &'a mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
    revoked_delegations: &'a HashSet<Vec<u8>>,
    trusted_root_ids: &'a [Vec<u8>],
    local_assurance_class: i32,
    exec_ctx: &'a CommandExecutionContext,
    delegation_use_counts: &'a HashMap<Vec<u8>, u64>,
    delegation_rate_events_ms: &'a HashMap<Vec<u8>, Vec<i64>>,
    now_ms: i64,
) -> CommandValidationContext<'a> {
    let local_node_id = signer.node_id().0;
    let location_classes: Vec<&str> = exec_ctx
        .location_classes
        .iter()
        .map(String::as_str)
        .collect();

    CommandValidationContext {
        local_node_id: &local_node_id,
        replay_cache,
        revoked_delegation_ids: revoked_delegations,
        delegation_use_counts,
        delegation_rate_events_ms,
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
    }
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
    let delegation_use_counts: HashMap<Vec<u8>, u64> = HashMap::new();
    let delegation_rate_events_ms: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();

    let ctx = build_validation_context(
        signer,
        replay_cache,
        revoked_delegations,
        trusted_root_ids,
        local_assurance_class,
        exec_ctx,
        &delegation_use_counts,
        &delegation_rate_events_ms,
        now_ms,
    );
    validate_command(command, &ctx)
}
