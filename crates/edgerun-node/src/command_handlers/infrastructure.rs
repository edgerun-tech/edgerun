//! Infrastructure command handlers — bootstrap, reachability, query.

use crate::command_dispatch::{
    build_command_result_payload, record_and_respond, record_and_respond_with_result_object,
    CommandDispatchResult, ControllerSet,
};
use crate::command_dispatch_event::record_action_event;
use edgerun_core::protocol::{
    command_envelope, AddBootstrapNodePayload, AddReachabilityHintPayload, CommandEnvelope,
    EventType, QueryNodeStatePayload,
};
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;

pub fn dispatch_add_bootstrap_node(
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
                "missing_add_bootstrap_node_payload",
                Vec::new(),
                None,
            );
        }
    };

    let bootstrap_payload = match AddBootstrapNodePayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_add_bootstrap_node_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let address = &bootstrap_payload.address;
    let label = if bootstrap_payload.label.is_empty() {
        address
    } else {
        &bootstrap_payload.label
    };

    edgerun_log::info!("add_bootstrap_node: label={} address={}", label, address);

    let result_payload = build_command_result_payload(
        command, 1, // CommandDecision::Committed
        "", None, None, None,
    );
    let response_bytes =
        crate::command_result_wire_codec::encode_command_result_payload(&result_payload);

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

pub fn dispatch_add_reachability_hint(
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
                "missing_add_reachability_hint_payload",
                Vec::new(),
                None,
            );
        }
    };

    let hint_payload = match AddReachabilityHintPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_add_reachability_hint_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let hint_type = hint_payload.hint_type.as_str();
    let value = &hint_payload.value;

    edgerun_log::info!("add_reachability_hint: type={} value={}", hint_type, value);

    let result_payload = build_command_result_payload(
        command, 1, // CommandDecision::Committed
        "", None, None, None,
    );
    let response_bytes =
        crate::command_result_wire_codec::encode_command_result_payload(&result_payload);

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

pub fn dispatch_query_node_state(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let _payload_bytes = match &command.payload {
        Some(command_envelope::Payload::InlinePayload(bytes)) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_query_node_state_payload",
                Vec::new(),
                None,
            );
        }
    };

    // Decode but we mainly just acknowledge the query
    let _query_payload = match QueryNodeStatePayload::decode(_payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_query_node_state_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    edgerun_log::info!("query_node_state: acknowledged");

    // Start and complete action immediately for queries
    record_action_event(
        store,
        stream_id,
        signer,
        command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted,
        vec![],
    );
    record_action_event(
        store,
        stream_id,
        signer,
        command,
        2, // ACTION_STATUS_COMPLETED
        EventType::ActionCompleted,
        vec![],
    );

    let response = b"query acknowledged".to_vec();

    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        None,
    )
}
