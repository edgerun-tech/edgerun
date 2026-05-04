//! Command router.
//!
//! `command_dispatch.rs` remains the fallback implementation while command
//! groups are extracted. This router is the new front door.

pub use crate::command_dispatch::{
    create_node_genesis_payload, project_config, project_config_from_base, project_controller_set,
    sign_event_envelope, CommandDispatchResult, ControllerSet,
};

use crate::command_authority::{command_authority_gate, CommandGateDecision};
use crate::command_dispatch_event::{append_command_result_event, CommandResultEventWrite};
use crate::command_dispatch_server_resource::dispatch_server_resource_command_result;
use edgerun_core::collections::{HashMap, HashSet};
use edgerun_core::command::CommandExecutionContext;
use edgerun_core::protocol::{CommandDecision, CommandEnvelope, EventType};
use edgerun_hardware_signing::MeshSigner;
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
        return dispatch_server_resource_command_after_gate(
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

    crate::command_dispatch::dispatch_command(
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

fn dispatch_server_resource_command_after_gate(
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
    match command_authority_gate(
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
    ) {
        CommandGateDecision::Accept => {
            dispatch_server_resource_command_result(command, store, stream_id, signer)
        }
        CommandGateDecision::CommitDuplicate => {
            append_decision(command, store, stream_id, signer, true, "duplicate_command")
        }
        CommandGateDecision::Reject(reason) => {
            append_decision(command, store, stream_id, signer, false, &reason)
        }
    }
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
