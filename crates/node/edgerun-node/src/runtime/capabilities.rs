use alloc::vec::Vec;

use edgerun_protocols::wire::{
    sdk_wire_bytes, CapabilityRequest, CapabilityResponse, SdkWireRecord, CAPABILITY_KIND_SIGNING,
    CAPABILITY_KIND_STORAGE, CAPABILITY_OPERATION_READ, CAPABILITY_OPERATION_SIGN,
    CAPABILITY_OPERATION_WRITE, CAPABILITY_STATUS_INVALID_REQUEST, CAPABILITY_STATUS_OK,
    CAPABILITY_STATUS_POLICY_DENIED, RUNTIME_EVENT_CAPABILITY_DENIED,
    RUNTIME_EVENT_CAPABILITY_EXECUTED, RUNTIME_SESSION_STATUS_OPEN, SDK_WIRE_ABI_VERSION,
};
use edgerun_work::capability_packet::{
    verify_capability_envelope_shape, CapabilityEnvelope, CAPABILITY_CONTENT_OBJECT,
    CAPABILITY_OPERATION_OBJECT_GET, CAPABILITY_OPERATION_OBJECT_PUT, CAPABILITY_PACKET_INVOKE,
};

use crate::storage::RuntimeStorage;

use super::{
    capability_denial_proof, sha256, signing_capability_input, signing_response_payload,
    signing_response_proof, storage_response_proof, storage_write_receipt_payload,
    RuntimeAppSigner, RuntimeError, RuntimeKernel, RuntimeSigner,
};

impl<S, G, K> RuntimeKernel<S, G, K>
where
    S: RuntimeStorage,
    G: RuntimeSigner,
    K: RuntimeAppSigner,
{
    pub fn invoke_storage_wire(
        &mut self,
        request_bytes: &[u8],
        provider: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, RuntimeError> {
        let request = decode_capability_request_wire(request_bytes)?;
        let response = self.invoke_storage(request_bytes, &request, provider, time)?;
        Ok(sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response)))
    }

    pub fn invoke_storage_envelope(
        &mut self,
        envelope: &CapabilityEnvelope,
        provider: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, RuntimeError> {
        if !storage_envelope_shape_is_supported(envelope) {
            return Err(RuntimeError::InvalidWireRecord);
        }
        let request = decode_capability_request_wire(&envelope.payload)?;
        if !self.storage_envelope_session_is_live(envelope, &request, time) {
            let response = self.denied_storage_response(&envelope.payload, &request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response)));
        }
        let response = self.invoke_storage(&envelope.payload, &request, provider, time)?;
        Ok(sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response)))
    }

    pub fn invoke_signing_wire(
        &mut self,
        request_bytes: &[u8],
        provider: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, RuntimeError> {
        let request = decode_capability_request_wire(request_bytes)?;
        let response = self.invoke_signing(request_bytes, &request, provider, time)?;
        Ok(sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response)))
    }

    fn invoke_storage(
        &mut self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
        time: u64,
    ) -> Result<CapabilityResponse, RuntimeError> {
        if request.capability_kind != CAPABILITY_KIND_STORAGE {
            let response = self.denied_storage_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }
        if !self.storage_request_is_bound(
            request.app_id,
            request.release_id,
            &request.context,
            request.operation,
            request.assurance,
            time,
        ) {
            let response = self.denied_storage_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }

        let payload = match request.operation {
            CAPABILITY_OPERATION_WRITE => {
                if request.payload_sha256 != sha256(&request.payload) {
                    return Ok(CapabilityResponse {
                        abi_version: SDK_WIRE_ABI_VERSION,
                        flags: 1,
                        capability_kind: CAPABILITY_KIND_STORAGE,
                        operation: request.operation,
                        status: CAPABILITY_STATUS_INVALID_REQUEST,
                        assurance: request.assurance,
                        request_sha256: sha256(request_bytes),
                        provider: provider.to_vec(),
                        responder: self.signer.runtime_id().to_vec(),
                        payload: b"payload_sha256_mismatch".to_vec(),
                        proof: Vec::new(),
                    });
                }
                self.storage
                    .write(&request.context, &request.subject_sha256, &request.payload)?;
                storage_write_receipt_payload(&request.payload)
            }
            CAPABILITY_OPERATION_READ => {
                let value = self
                    .storage
                    .read(&request.context, &request.subject_sha256)?
                    .unwrap_or_default();
                if request.payload_sha256 != [0u8; 32] && request.payload_sha256 != sha256(&value) {
                    return Ok(CapabilityResponse {
                        abi_version: SDK_WIRE_ABI_VERSION,
                        flags: 1,
                        capability_kind: CAPABILITY_KIND_STORAGE,
                        operation: request.operation,
                        status: CAPABILITY_STATUS_INVALID_REQUEST,
                        assurance: request.assurance,
                        request_sha256: sha256(request_bytes),
                        provider: provider.to_vec(),
                        responder: self.signer.runtime_id().to_vec(),
                        payload: b"read_payload_sha256_mismatch".to_vec(),
                        proof: Vec::new(),
                    });
                }
                value
            }
            _ => {
                let response = self.denied_storage_response(request_bytes, request, provider);
                self.append_event(
                    time,
                    RUNTIME_EVENT_CAPABILITY_DENIED,
                    CAPABILITY_STATUS_POLICY_DENIED,
                    sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
                );
                return Ok(response);
            }
        };
        let proof = storage_response_proof(request_bytes, request.operation, provider, &payload);
        let response = CapabilityResponse::ok(
            sha256(request_bytes),
            CAPABILITY_KIND_STORAGE,
            request.operation,
            request.assurance,
            provider.to_vec(),
            self.signer.runtime_id().to_vec(),
            payload,
            proof.to_vec(),
        );
        self.append_event(
            time,
            RUNTIME_EVENT_CAPABILITY_EXECUTED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
        );
        Ok(response)
    }

    fn storage_envelope_session_is_live(
        &self,
        envelope: &CapabilityEnvelope,
        request: &CapabilityRequest,
        time: u64,
    ) -> bool {
        let Some(session) = self.capability_sessions.get(&envelope.session_id) else {
            return false;
        };
        session.capability_id == envelope.capability_id
            && session.provider_node_id == envelope.target_node_id
            && envelope.target_node_id == self.signer.runtime_id()
            && session.app_id == request.app_id
            && session.release_id == request.release_id
            && session.status == RUNTIME_SESSION_STATUS_OPEN
            && session.valid_until >= time
            && match envelope.operation {
                CAPABILITY_OPERATION_OBJECT_GET => request.operation == CAPABILITY_OPERATION_READ,
                CAPABILITY_OPERATION_OBJECT_PUT => request.operation == CAPABILITY_OPERATION_WRITE,
                _ => false,
            }
    }

    fn invoke_signing(
        &mut self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
        time: u64,
    ) -> Result<CapabilityResponse, RuntimeError> {
        if request.capability_kind != CAPABILITY_KIND_SIGNING
            || request.operation != CAPABILITY_OPERATION_SIGN
        {
            let response = self.denied_capability_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }
        let allowed = self.apps.get(&request.app_id).is_some_and(|app| {
            app.release_id == request.release_id
                && request.payload_sha256 == sha256(&request.payload)
        }) && self.capability_is_granted(
            request.app_id,
            request.release_id,
            sha256(&request.context),
            CAPABILITY_KIND_SIGNING,
            CAPABILITY_OPERATION_SIGN,
            request.assurance,
            time,
        );
        if !allowed {
            let response = self.denied_capability_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }
        let public_key = match self.app_signer.app_public_key(&request.app_id)? {
            Some(public_key) => public_key,
            None => {
                let response = self.denied_capability_response(request_bytes, request, provider);
                self.append_event(
                    time,
                    RUNTIME_EVENT_CAPABILITY_DENIED,
                    CAPABILITY_STATUS_POLICY_DENIED,
                    sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
                );
                return Ok(response);
            }
        };
        let signature = self
            .app_signer
            .sign_app_payload(&request.app_id, &signing_capability_input(request))?;
        let payload = signing_response_payload(&public_key, &signature);
        let proof = signing_response_proof(request_bytes, provider, &public_key, &signature);
        let response = CapabilityResponse::ok(
            sha256(request_bytes),
            CAPABILITY_KIND_SIGNING,
            CAPABILITY_OPERATION_SIGN,
            request.assurance,
            provider.to_vec(),
            self.signer.runtime_id().to_vec(),
            payload,
            proof.to_vec(),
        );
        self.append_event(
            time,
            RUNTIME_EVENT_CAPABILITY_EXECUTED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
        );
        Ok(response)
    }

    fn denied_storage_response(
        &self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
    ) -> CapabilityResponse {
        self.denied_capability_response(request_bytes, request, provider)
    }

    fn denied_capability_response(
        &self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
    ) -> CapabilityResponse {
        let reason = b"policy_denied".to_vec();
        let proof = capability_denial_proof(
            request_bytes,
            CAPABILITY_STATUS_POLICY_DENIED,
            provider,
            &reason,
        );
        CapabilityResponse::denied(
            sha256(request_bytes),
            request.capability_kind,
            request.operation,
            request.assurance,
            provider.to_vec(),
            self.signer.runtime_id().to_vec(),
            reason,
            proof.to_vec(),
        )
    }
}

fn storage_envelope_shape_is_supported(envelope: &CapabilityEnvelope) -> bool {
    verify_capability_envelope_shape(envelope)
        && envelope.kind == CAPABILITY_PACKET_INVOKE
        && envelope.content_type == CAPABILITY_CONTENT_OBJECT
        && matches!(
            envelope.operation,
            CAPABILITY_OPERATION_OBJECT_GET | CAPABILITY_OPERATION_OBJECT_PUT
        )
}

fn decode_capability_request_wire(request_bytes: &[u8]) -> Result<CapabilityRequest, RuntimeError> {
    let owned = request_bytes.to_vec();
    match edgerun_protocols::wire::from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(
        &owned,
    )
    .map_err(|_| RuntimeError::InvalidWireRecord)?
    {
        SdkWireRecord::CapabilityRequest(request) => Ok(request),
        _ => Err(RuntimeError::InvalidWireRecord),
    }
}
