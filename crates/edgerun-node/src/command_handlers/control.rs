//! Control command handlers — add/remove/transfer controller.

use crate::command_dispatch::{
    extract_identity_from_command, record_and_respond, CommandDispatchResult, ControllerSet,
};
use crate::command_dispatch_event::record_action_event;
use edgerun_core::util::bytes_to_hex_prefixed;
use edgerun_hardware_signing::MeshSigner;
use edgerun_core::protocol::{CommandEnvelope, EventType};
use edgerun_storage::NodeStore;

pub fn dispatch_add_controller(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    // Extract the new controller identity from the command's payload
    let new_controller_id = extract_identity_from_command(command);
    if new_controller_id.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_controller_identity",
            Vec::new(),
            None,
        );
    }

    // Add to controller set
    controllers.add(new_controller_id.clone());

    edgerun_log::info!("controller added");

    let response = format!(
        "controller added: {}",
        bytes_to_hex_prefixed(&new_controller_id)
    )
    .into_bytes();
    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        Some((&bytes_to_hex_prefixed(&new_controller_id), "added")),
    )
}

pub fn dispatch_remove_controller(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let target_id = extract_identity_from_command(command);
    if target_id.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_controller_identity",
            Vec::new(),
            None,
        );
    }

    if !controllers.remove(&target_id) {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "controller_not_found",
            Vec::new(),
            None,
        );
    }

    // Guard: never allow removing the last controller — node would be orphaned
    if controllers.to_vec().is_empty() {
        controllers.add(target_id); // revert
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "cannot_remove_last_controller",
            Vec::new(),
            None,
        );
    }

    edgerun_log::info!("controller removed");

    let response =
        format!("controller removed: {}", bytes_to_hex_prefixed(&target_id)).into_bytes();
    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        Some((&bytes_to_hex_prefixed(&target_id), "removed")),
    )
}

pub fn dispatch_transfer_control(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let new_controller_id = extract_identity_from_command(command);
    if new_controller_id.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_controller_identity",
            Vec::new(),
            None,
        );
    }

    // Get current controller (issuer) — the one making the transfer request
    let current_controller = command
        .issuer
        .as_ref()
        .map(|i| i.identity_id.clone())
        .unwrap_or_default();

    if current_controller.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_current_controller",
            Vec::new(),
            None,
        );
    }

    // Remove old, add new
    controllers.remove(&current_controller);
    controllers.add(new_controller_id.clone());

    // Guard: never allow removing the last controller
    if controllers.to_vec().is_empty() {
        controllers.add(current_controller); // revert
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "cannot_remove_last_controller",
            Vec::new(),
            None,
        );
    }

    edgerun_log::info!("control transferred");

    let response = format!(
        "control transferred from {} to {}",
        bytes_to_hex_prefixed(&current_controller),
        bytes_to_hex_prefixed(&new_controller_id)
    )
    .into_bytes();

    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        Some((&bytes_to_hex_prefixed(&new_controller_id), "transferred")),
    )
}
