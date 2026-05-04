//! User command handlers — user presence/signature requests.

use crate::command_dispatch::{
    build_command_result_payload, extract_app_id_from_command, record_and_respond,
    record_and_respond_with_result_object, store_object_or_log, CommandDispatchResult,
    ControllerSet,
};
use crate::command_dispatch_event::record_action_event;
use edgerun_core::util::{now_unix_micros_u64, now_unix_millis_i64};
use edgerun_crypto::rand_core::RngCore;
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::stream::{
    command_envelope, CommandEnvelope, EventType, RequestSignaturePayload,
    RequestUserPresencePayload, UserPresenceGrantedPayload, UserPresenceRequestPayload,
};
use edgerun_storage::NodeStore;
use prost::Message;

pub fn dispatch_request_user_presence(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let payload_bytes = match &command.payload {
        Some(command_envelope::Payload::InlinePayload(bytes)) => bytes.clone(),
        _ => Vec::new(),
    };

    let req = if !payload_bytes.is_empty() {
        match RequestUserPresencePayload::decode(payload_bytes.as_slice()) {
            Ok(p) => p,
            Err(_) => {
                return record_and_respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    controllers,
                    false,
                    "invalid_presence_request_payload",
                    Vec::new(),
                    None,
                );
            }
        }
    } else {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_presence_request_payload",
            Vec::new(),
            None,
        );
    };

    if req.reason.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "empty_presence_reason",
            Vec::new(),
            None,
        );
    }

    let ttl = if req.ttl_seconds > 0 && req.ttl_seconds <= 300 {
        req.ttl_seconds
    } else {
        60
    };

    let mut token = [0u8; 32];
    edgerun_crypto::rand_core::RngCore::fill_bytes(&mut edgerun_crypto::rng::OsRng, &mut token);

    let expires_at = now_unix_micros_u64() + (ttl as u64 * 1_000_000);

    let app_id = extract_app_id_from_command(command);

    let request_payload = UserPresenceRequestPayload {
        payload_version: 1,
        app_id: app_id.clone(),
        reason: req.reason.clone(),
        session_id: req.session_id.clone(),
        ttl_seconds: ttl,
    };

    let request_bytes = Message::encode_to_vec(&request_payload);
    let request_obj = store.put_object(
        &request_bytes,
        edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
        &[stream_id.to_vec()],
    );

    let granted_payload = UserPresenceGrantedPayload {
        payload_version: 1,
        presence_token: token.to_vec(),
        app_id,
        session_id: req.session_id,
        expires_at: expires_at as i64,
    };

    let granted_bytes = Message::encode_to_vec(&granted_payload);
    let granted_obj = store.put_object(
        &granted_bytes,
        edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
        &[stream_id.to_vec()],
    );

    let granted_obj_ref = granted_obj.as_ref().ok().cloned();
    let result_payload = build_command_result_payload(
        command,
        1, // CommandDecision::Committed
        "",
        None,
        None,
        granted_obj_ref,
    );
    let response_bytes = prost::Message::encode_to_vec(&result_payload);

    // Write presence token to local session store for quick validation
    if let Some(ref token_store) = crate::get_local_session_store() {
        let _ = token_store.write_presence_token(
            &token,
            &granted_payload.app_id,
            granted_payload.expires_at,
        );
    }

    edgerun_log::info!(
        "user_presence_granted: session={} ttl={}s",
        req.session_id,
        ttl
    );

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

pub fn dispatch_request_signature(
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
                "missing_request_signature_payload",
                Vec::new(),
                None,
            );
        }
    };

    let sig_req = match RequestSignaturePayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_request_signature_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    if sig_req.digest_to_sign.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "empty_digest_to_sign",
            Vec::new(),
            None,
        );
    }

    let ttl = if sig_req.ttl_seconds > 0 && sig_req.ttl_seconds <= 300 {
        sig_req.ttl_seconds
    } else {
        60
    };

    let mut token = [0u8; 32];
    edgerun_crypto::rand_core::RngCore::fill_bytes(&mut edgerun_crypto::rng::OsRng, &mut token);

    let expires_at = now_unix_millis_i64() + (ttl as i64 * 1000);

    let app_id = extract_app_id_from_command(command);

    edgerun_log::info!(
        "signature_request: session={} ttl={}s",
        sig_req.session_id,
        ttl
    );

    let result_payload = build_command_result_payload(
        command, 1, // CommandDecision::Committed
        "", None, None, None,
    );
    let response_bytes = prost::Message::encode_to_vec(&result_payload);

    // Write signature request token to local session store
    if let Some(ref token_store) = crate::get_local_session_store() {
        let _ = token_store.write_signature_request(
            &token,
            &sig_req.digest_to_sign,
            &app_id,
            expires_at,
        );
    }

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
