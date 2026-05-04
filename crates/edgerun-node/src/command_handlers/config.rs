//! Config command handler — update config.

use crate::command_dispatch::{
    apply_config_patch, record_and_respond, record_and_respond_with_result_object,
    CommandDispatchResult, ControllerSet,
};
use crate::command_dispatch_event::record_action_event;
use edgerun_core::protocol::CommandEnvelope;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;

pub fn dispatch_update_config(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    // Extract config patch from payload (JSON format)
    let config_patch: Vec<u8> = match &command.payload {
        Some(edgerun_core::protocol::command_envelope::Payload::InlinePayload(bytes)) => {
            bytes.clone()
        }
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_config_patch",
                Vec::new(),
                None,
            );
        }
    };

    let mut projected = crate::command_dispatch::NodeConfig {
        stream_id: edgerun_core::util::bytes_to_hex(stream_id),
        name: None,
        controllers: vec![],
        trust_nodes: vec![],
        allowed_peers: vec![],
        bootstrap_peers: vec![],
        signer: None,
    };
    if let Err(reason) = apply_config_patch(&mut projected, &config_patch) {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "invalid_config_patch_json",
            Vec::new(),
            None,
        );
    }

    edgerun_log::info!("config update received");

    let patch_object = match store.put_object(
        &config_patch,
        edgerun_core::protocol::ObjectKind::DerivedView as i32,
        &[stream_id.to_vec()],
    ) {
        Ok(object_ref) => Some(object_ref),
        Err(e) => {
            edgerun_log::warn!("failed to store config patch object: {}", e);
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "storage_failed",
                Vec::new(),
                None,
            );
        }
    };

    let response = b"config_update_received".to_vec();
    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        None,
        patch_object,
    )
}
