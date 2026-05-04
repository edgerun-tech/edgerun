//! Identity command handlers — create/import identity.

use crate::command_dispatch::{
    build_command_result_payload, record_and_respond, record_and_respond_with_result_object,
    CommandDispatchResult, ControllerSet,
};
use crate::command_dispatch_event::record_action_event;
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::stream::{
    command_envelope, CommandEnvelope, CreateIdentityPayload, EventType, ImportIdentityPayload,
};
use edgerun_storage::NodeStore;
use prost::Message;

pub fn dispatch_create_identity(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let payload_bytes = match &command.payload {
        Some(command_envelope::Payload::InlinePayload(bytes)) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_create_identity_payload",
                Vec::new(),
                None,
            );
        }
    };

    let create_payload = match CreateIdentityPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_create_identity_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let label = if create_payload.label.is_empty() {
        "primary"
    } else {
        &create_payload.label
    };

    edgerun_log::info!("create_identity: label={}", label);

    let result_payload = build_command_result_payload(
        command, 1, // CommandDecision::Committed
        "", None, None, None,
    );
    let response_bytes = prost::Message::encode_to_vec(&result_payload);

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        None,
    )
}

pub fn dispatch_import_identity(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let payload_bytes = match &command.payload {
        Some(command_envelope::Payload::InlinePayload(bytes)) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_import_identity_payload",
                Vec::new(),
                None,
            );
        }
    };

    let import_payload = match ImportIdentityPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_import_identity_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let label = if import_payload.label.is_empty() {
        "imported"
    } else {
        &import_payload.label
    };

    edgerun_log::info!("import_identity: label={}", label);

    let result_payload = build_command_result_payload(
        command, 1, // CommandDecision::Committed
        "", None, None, None,
    );
    let response_bytes = prost::Message::encode_to_vec(&result_payload);

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        None,
    )
}
