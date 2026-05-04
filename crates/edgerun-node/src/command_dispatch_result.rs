//! Shared command result helpers.

use edgerun_core::wire_command::command_hash;
use edgerun_proto::edgerun::v0::common::{CommandRef, ObjectRef};
use edgerun_proto::edgerun::v0::stream::{CommandEnvelope, CommandResultPayload};

pub fn command_ref_from(command: &CommandEnvelope) -> CommandRef {
    CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    }
}

pub fn build_command_result_payload(
    command: &CommandEnvelope,
    decision: i32,
    reason_code: &str,
    decision_basis: Option<ObjectRef>,
    effect_summary_object: Option<ObjectRef>,
    result_object: Option<ObjectRef>,
) -> CommandResultPayload {
    CommandResultPayload {
        payload_version: 1,
        command: Some(command_ref_from(command)),
        issuer: command.issuer.clone(),
        decision,
        decision_basis,
        reason_code: reason_code.to_string(),
        effect_summary_object,
        result_object,
    }
}
