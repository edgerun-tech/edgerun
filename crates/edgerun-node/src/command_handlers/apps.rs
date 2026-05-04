//! App command handlers — install/uninstall.

use crate::app_package_wire_codec::decode_app_package;
use crate::command_dispatch::{
    build_command_result_payload, extract_payload_or_reject, record_and_respond,
    record_and_respond_with_result_object, CommandDispatchResult, ControllerSet,
};
use crate::command_result_wire_codec::encode_command_result_payload;
use edgerun_core::util::{bytes_to_hex, now_unix_millis_i64};
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::stream::{
    CommandEnvelope, CommandType, InstallAppPayload, UninstallAppPayload,
};
use edgerun_storage::NodeStore;

pub fn dispatch_install_app(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let reject_fn = |e: String| -> CommandDispatchResult {
        record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            &e,
            Vec::new(),
            None,
        )
    };

    let install_payload: InstallAppPayload =
        match extract_payload_or_reject(command, "install_app_payload", &reject_fn) {
            Ok(p) => p,
            Err(result) => return result,
        };

    let package_object_ref = match install_payload.app_package {
        Some(ref obj) => obj.clone(),
        None => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "app_package_required",
                Vec::new(),
                None,
            );
        }
    };

    let package_bytes = match store.get_object(&package_object_ref) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "app_package_not_found",
                Vec::new(),
                None,
            );
        }
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("app_package_not_found: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let domain = if install_payload.domain.is_empty() {
        "default".to_string()
    } else {
        install_payload.domain.clone()
    };

    edgerun_log::info!("install_app: domain={}", domain);

    let app_package = match decode_app_package(package_bytes.content.as_slice()) {
        Ok(pkg) => pkg,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_app_package: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    if let Some(ref wasm_ref) = app_package.wasm_object {
        edgerun_log::info!(
            "install_app: wasm_object kind={:?} id={}",
            wasm_ref.object_kind,
            bytes_to_hex(&wasm_ref.object_id)
        );
    }
    for (name, asset_ref) in &app_package.assets {
        edgerun_log::info!(
            "install_app: asset '{}' kind={:?} id={}",
            name,
            asset_ref.object_kind,
            bytes_to_hex(&asset_ref.object_id)
        );
    }

    let result_payload = build_command_result_payload(
        command,
        CommandType::InstallApp as i32,
        "",
        None,
        None,
        Some(package_object_ref),
    );
    let response_bytes = encode_command_result_payload(&result_payload);

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
        Some(result_payload.result_object.unwrap()),
    )
}

pub fn dispatch_uninstall_app(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let reject_fn = |e: String| -> CommandDispatchResult {
        record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            &e,
            Vec::new(),
            None,
        )
    };

    let uninstall_payload: UninstallAppPayload =
        match extract_payload_or_reject(command, "uninstall_app_payload", &reject_fn) {
            Ok(p) => p,
            Err(result) => return result,
        };

    let app_id = if !uninstall_payload.app_id.is_empty() {
        uninstall_payload.app_id.clone()
    } else {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "app_id_required",
            Vec::new(),
            None,
        );
    };

    edgerun_log::info!(
        "uninstall_app: app_id={}",
        bytes_to_hex(&app_id)
    );

    let mut found = false;
    if let Ok(Some(_app)) = store.get_app(&bytes_to_hex(&app_id)) {
        found = true;
        edgerun_log::info!("uninstall_app: found app, marking uninstalled");

        if let Err(e) = store.uninstall_app(&bytes_to_hex(&app_id), now_unix_millis_i64()) {
            edgerun_log::warn!("failed to mark app uninstalled: {}", e);
        }
    }

    if !found {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "app_not_found",
            Vec::new(),
            None,
        );
    }

    let response = format!("app uninstalled: {}", bytes_to_hex(&app_id)).into_bytes();

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
