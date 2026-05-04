//! Protocol dispatch loop: serve_one() and pump_one_event().

use crate::prelude::v1::*;
use edgerun_capabilities::CapabilityError;
use edgerun_core::protocol::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_core::protocol::capability_runtime::{
    capability_remote_envelope, CapabilityRemoteEnvelope, CapabilityResultFrame,
};

use super::{RemoteCapabilityProvider, RemoteCapabilityTransport, RemoteInvocationResult};

/// Handle one envelope from the transport. Returns `Ok(false)` on EOF.
pub fn serve_one<P: RemoteCapabilityProvider, T: RemoteCapabilityTransport>(
    provider: &mut P,
    transport: &mut T,
) -> Result<bool, CapabilityError> {
    let Some(envelope) = transport.recv()? else {
        return Ok(false);
    };
    match envelope.message {
        Some(capability_remote_envelope::Message::SessionOpen(open)) => {
            let accept = provider.open_session(&open)?;
            transport.send(CapabilityRemoteEnvelope {
                message: Some(capability_remote_envelope::Message::SessionAccept(accept)),
            })?;
        }
        Some(capability_remote_envelope::Message::Invocation(invocation)) => {
            let message = match provider.invoke(&invocation.grant_id, &invocation, None) {
                Ok(response) => result_to_message(response),
                Err(err) => capability_remote_envelope::Message::Result(capability_error_result(
                    &invocation,
                    err,
                )),
            };
            transport.send(CapabilityRemoteEnvelope {
                message: Some(message),
            })?;
        }
        Some(capability_remote_envelope::Message::InvocationFrame(frame)) => {
            let Some(invocation) = frame.invocation else {
                return Err(CapabilityError::InvalidRequest(
                    "remote invocation frame must contain an invocation",
                ));
            };
            let message = match provider.invoke(
                &invocation.grant_id,
                &invocation,
                Some(&frame.inline_parameters),
            ) {
                Ok(response) => result_to_message(response),
                Err(err) => capability_remote_envelope::Message::Result(capability_error_result(
                    &invocation,
                    err,
                )),
            };
            transport.send(CapabilityRemoteEnvelope {
                message: Some(message),
            })?;
        }
        // SessionClose is fire-and-forget — no response envelope required.
        Some(capability_remote_envelope::Message::SessionClose(close)) => {
            provider.close_session(&close)?;
        }
        Some(capability_remote_envelope::Message::Request(request)) => {
            if let Some(grant) = provider.handle_request(&request)? {
                transport.send(CapabilityRemoteEnvelope {
                    message: Some(capability_remote_envelope::Message::Grant(grant)),
                })?;
            }
        }
        Some(capability_remote_envelope::Message::Grant(grant)) => {
            provider.handle_grant(&grant)?;
        }
        Some(capability_remote_envelope::Message::Revocation(revocation)) => {
            provider.handle_revocation(&revocation)?;
        }
        _ => {}
    }
    Ok(true)
}

/// Emit one session event from the provider. Returns `Ok(false)` if no events pending.
pub fn pump_one_event<P: RemoteCapabilityProvider, T: RemoteCapabilityTransport>(
    provider: &mut P,
    transport: &mut T,
    session_id: &[u8],
) -> Result<bool, CapabilityError> {
    let Some(event) = provider.next_event(session_id)? else {
        return Ok(false);
    };
    transport.send(CapabilityRemoteEnvelope {
        message: Some(capability_remote_envelope::Message::SessionEvent(event)),
    })?;
    Ok(true)
}

fn result_to_message(result: RemoteInvocationResult) -> capability_remote_envelope::Message {
    if result.inline_payload.is_empty() {
        capability_remote_envelope::Message::Result(result.result)
    } else {
        capability_remote_envelope::Message::ResultFrame(CapabilityResultFrame {
            result: Some(result.result),
            inline_payload: result.inline_payload,
        })
    }
}

/// Build a failed `CapabilityResult` from an error.
pub fn capability_error_result(
    invocation: &CapabilityInvocation,
    error: CapabilityError,
) -> CapabilityResult {
    CapabilityResult {
        result_version: 1,
        invocation_id: invocation.invocation_id.clone(),
        grant_id: invocation.grant_id.clone(),
        success: false,
        result_access_class: invocation.requested_access_class,
        produced_event_kinds: Vec::new(),
        payload_object: None,
        error_reason: error.to_string(),
        produced_at: None,
        signature: None,
    }
}
