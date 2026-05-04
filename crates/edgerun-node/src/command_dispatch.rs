//! Command dispatch for the edgerun node daemon.
//!
//! Replaces the old signature-only validation with:
//! - Full command validation (replay, timing, delegation signature verification)
//! - Command type dispatch (ADD_CONTROLLER, REMOVE_CONTROLLER, TRANSFER_CONTROL, etc.)
//! - Controller set management and projection from event log
//! - Revocation record processing

// Handler modules — split from monolithic command_dispatch.rs
pub mod command_handlers;

// Re-export key types for handlers
pub use command_handlers::{
    control, custom, apps, identity, infrastructure, user, config,
};

use crate::command_authority::{build_validation_context, check_replay_cache, CommandGateDecision};
use crate::command_dispatch_payload::extract_payload_or_reject;
use crate::config::NodeConfig;
use edgerun_core::collections::{HashMap, HashSet};
use edgerun_core::command::{
    command_hash, validate_command, CommandExecutionContext, CommandValidationContext,
};
use edgerun_core::encrypted_envelope::validate_encrypted_envelope;
use edgerun_proto::edgerun::v0::common::EncryptedEnvelope;
use edgerun_core::result::Verdict;
use edgerun_core::util::{
    now_prost_timestamp, now_unix_micros_u64, now_unix_millis_i64, now_unix_secs_i64,
};
use edgerun_core::validators_proto::{
    validate_control_change_command, validate_delegation_chain, validate_revocation_record,
};
use edgerun_crypto::rand_core::RngCore;
use edgerun_hardware_signing::MeshSigner;
use edgerun_json::Value as JsonValue;
use edgerun_proto::edgerun::v0::common::{CommandRef, EventRef, ObjectRef};
use edgerun_proto::edgerun::v0::stream::{
    CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload,
    CommandType, EventType,
};
use edgerun_proto::edgerun::v0::trust::{
    DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord,
};
use edgerun_storage::NodeStore;
use prost::Message;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Controller state
// ---------------------------------------------------------------------------

/// The current set of controller identities authorized to influence this node.
#[derive(Clone, Debug, Default)]
pub struct ControllerSet {
    controllers: HashSet<Vec<u8>>,
}

impl ControllerSet {
    pub fn new(initial: Vec<Vec<u8>>) -> Self {
        Self {
            controllers: initial.into_iter().collect(),
        }
    }

    pub fn add(&mut self, identity_id: Vec<u8>) {
        self.controllers.insert(identity_id);
    }

    pub fn remove(&mut self, identity_id: &Vec<u8>) -> bool {
        self.controllers.remove(identity_id)
    }

    pub fn contains(&self, identity_id: &Vec<u8>) -> bool {
        self.controllers.contains(identity_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.controllers.iter()
    }

    pub fn controller_ids(&self) -> Vec<Vec<u8>> {
        self.controllers.iter().cloned().collect()
    }

    pub fn to_vec(&self) -> Vec<Vec<u8>> {
        self.controllers.iter().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Command dispatch result
// ---------------------------------------------------------------------------

pub struct CommandDispatchResult {
    pub event_type: EventType,
    pub decision: i32, // COMMAND_DECISION_COMMITTED = 1, REJECTED = 2
    pub reason_code: String,
    pub response_bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Processes a command through full validation and type-specific dispatch.
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
    let now_ms = now_unix_millis_i64();
    let local_node_id = signer.node_id().0;

    // Check replay cache (both persistent and in-memory)
    if let Some(CommandGateDecision::CommitDuplicate) =
        check_replay_cache(command, store, replay_cache)
    {
        return record_and_respond(
            command, store, stream_id, signer, controllers,
            true, "duplicate_command", Vec::new(), None,
        );
    }

    // Build validation context from execution context + node-local state
    let delegation_use_counts: HashMap<Vec<u8>, u64> = HashMap::new();
    let delegation_rate_events_ms: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();

    let ctx = build_validation_context(
        signer, replay_cache, revoked_delegations, trusted_root_ids,
        local_assurance_class, exec_ctx,
        &delegation_use_counts, &delegation_rate_events_ms, now_ms,
    );

    // Run full validation
    let validation_result = validate_command(command, &ctx);

    // If validation rejected or deferred, record and return
    match validation_result.verdict {
        Verdict::Reject => {
            let reason = validation_result
                .reason_code
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "rejected".into());
            return record_and_respond(
                command, store, stream_id, signer, controllers,
                false, &reason, Vec::new(), None,
            );
        }
        Verdict::Defer => {
            return record_and_respond(
                command, store, stream_id, signer, controllers,
                false, "deferred", Vec::new(), None,
            );
        }
        Verdict::Duplicate => {
            return record_and_respond(
                command, store, stream_id, signer, controllers,
                true, "duplicate_command", Vec::new(), None,
            );
        }
        Verdict::Accept => {}
    }

    // Policy check
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
        return record_and_respond(
            command, store, stream_id, signer, controllers,
            false, &reason, Vec::new(), None,
        );
    }

    // Control change validation
    if matches!(
        CommandType::from_i32(command.command_type),
        Some(
            CommandType::AddController
                | CommandType::RemoveController
                | CommandType::TransferControl
        )
    ) {
        let current_controllers = controllers.to_vec();
        let control_result = validate_control_change_command(command, &current_controllers, 1);
        if control_result.verdict != Verdict::Accept {
            let reason = control_result
                .reason_code
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "control_change_rejected".into());
            return record_and_respond(
                command, store, stream_id, signer, controllers,
                false, &reason, Vec::new(), None,
            );
        }
    }

    // Encrypted envelope validation
    let needs_encryption = matches!(
        CommandType::from_i32(command.command_type),
        Some(
            CommandType::StoreObject
                | CommandType::ExecuteWorkload
        )
    );

    if needs_encryption {
        let payload_bytes = match &command.payload {
            Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(b)) => Some(b.clone()),
            _ => None,
        };

        if let Some(raw) = payload_bytes {
            match EncryptedEnvelope::decode(raw.as_slice()) {
                Ok(env) => {
                    if let Err(reason) = validate_encrypted_envelope(&env) {
                        return record_and_respond(
                            command, store, stream_id, signer, controllers,
                            false, &reason, Vec::new(), None,
                        );
                    }
                }
                Err(_) => {
                    return record_and_respond(
                        command, store, stream_id, signer, controllers,
                        false, "PLAINTEXT_PAYLOAD_REJECTED", Vec::new(), None,
                    );
                }
            }
        }
    }

    // Dispatch by command type — uses handler modules in command_handlers/
    let command_type = command.command_type;
    match command_type {
        x if x == CommandType::AddController as i32 => {
            control::dispatch_add_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RemoveController as i32 => {
            control::dispatch_remove_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::TransferControl as i32 => {
            control::dispatch_transfer_control(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::PublishSnapshot as i32 => {
            record_and_respond(
                command, store, stream_id, signer, controllers,
                false, "use_produce_snapshot_request", Vec::new(), None,
            )
        }
        x if x == CommandType::FetchObject as i32 => {
            record_and_respond(
                command, store, stream_id, signer, controllers,
                false, "use_fetch_object_request", Vec::new(), None,
            )
        }
        x if x == CommandType::Query as i32 => {
            record_and_respond(
                command, store, stream_id, signer, controllers,
                true, "", Vec::new(), None,
            )
        }
        x if x == CommandType::ExecuteWorkload as i32 => record_and_respond(
            command, store, stream_id, signer, controllers,
            false, "execute_workload_not_supported", Vec::new(), None,
        ),
        x if x == CommandType::TerminateWorkload as i32 => record_and_respond(
            command, store, stream_id, signer, controllers,
            false, "terminate_workload_not_supported", Vec::new(), None,
        ),
        x if x == CommandType::CreateDelegation as i32
            || x == CommandType::CreateRevocation as i32 =>
        {
            custom::dispatch_custom_command(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::UpdateConfig as i32 => {
            config::dispatch_update_config(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::InstallApp as i32 => {
            apps::dispatch_install_app(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::UninstallApp as i32 => {
            apps::dispatch_uninstall_app(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::CreateIdentity as i32 => {
            identity::dispatch_create_identity(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::ImportIdentity as i32 => {
            identity::dispatch_import_identity(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::AddBootstrapNode as i32 => {
            infrastructure::dispatch_add_bootstrap_node(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::AddReachabilityHint as i32 => {
            infrastructure::dispatch_add_reachability_hint(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::QueryNodeState as i32 => {
            infrastructure::dispatch_query_node_state(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RequestUserPresence as i32 => {
            user::dispatch_request_user_presence(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RequestSignature as i32 => {
            user::dispatch_request_signature(command, store, stream_id, signer, controllers)
        }
        _ => {
            if command_type > 0 && command_type < 1000 {
                record_and_respond(
                    command, store, stream_id, signer, controllers,
                    false, "unknown_command_type_reserved", Vec::new(), None,
                )
            } else {
                record_and_respond(
                    command, store, stream_id, signer, controllers,
                    false, "unsupported_extension_command_type", Vec::new(), None,
                )
            }
        }
    }
}
