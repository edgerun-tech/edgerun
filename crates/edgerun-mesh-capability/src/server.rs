use crate::prelude::v1::*;
use edgerun_capabilities::CapabilityError;
use edgerun_hardware_signing::NodeID;
#[allow(unused_imports)] // used in tests
use edgerun_mesh::{FrameType, MeshFrame, MeshFrameHeader};
use edgerun_mesh_link::MeshLink;
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityRemoteEnvelope,
};
use edgerun_remote_capability::RemoteCapabilityProvider;
use prost::Message;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use super::*;

// MeshCapabilityServer — serves local providers over the mesh
// ---------------------------------------------------------------------------

/// Hosts a `RemoteCapabilityProvider` and processes inbound capability
/// requests received via mesh frames.
pub struct MeshCapabilityServer<P> {
    provider: P,
    dispatcher: MeshEnvelopeDispatcher,
}

impl<P: RemoteCapabilityProvider> MeshCapabilityServer<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            dispatcher: MeshEnvelopeDispatcher::new(),
        }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    pub fn dispatcher(&mut self) -> &mut MeshEnvelopeDispatcher {
        &mut self.dispatcher
    }

    /// Processes one inbound envelope for the server, if available.
    pub fn serve_one(&mut self, _link: &mut MeshLink) -> Result<bool, CapabilityError> {
        let senders: Vec<NodeID> = self.dispatcher.inboxes_mut().keys().copied().collect();

        for sender in senders {
            if let Some(inboxes) = self.dispatcher.inboxes_mut().get_mut(&sender) {
                if let Some(inbox) = inboxes.first_mut() {
                    if let Some(envelope) = inbox.pop() {
                        let response = self.process_envelope(&sender, envelope)?;
                        if let Some(resp_env) = response {
                            if let Some(resp_inboxes) =
                                self.dispatcher.inboxes_mut().get_mut(&sender)
                            {
                                if let Some(resp_inbox) = resp_inboxes.first_mut() {
                                    resp_inbox.push(resp_env);
                                }
                            }
                        }
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    pub(crate) fn process_envelope(
        &mut self,
        _sender: &NodeID,
        envelope: CapabilityRemoteEnvelope,
    ) -> Result<Option<CapabilityRemoteEnvelope>, CapabilityError> {
        match envelope.message {
            Some(capability_remote_envelope::Message::SessionOpen(open)) => {
                let accept = self.provider.open_session(&open)?;
                Ok(Some(CapabilityRemoteEnvelope {
                    message: Some(capability_remote_envelope::Message::SessionAccept(accept)),
                }))
            }
            Some(capability_remote_envelope::Message::Invocation(invocation)) => {
                let result = self
                    .provider
                    .invoke(&invocation.grant_id, &invocation, None);
                let msg = match result {
                    Ok(resp) => capability_remote_envelope::Message::Result(resp.result),
                    Err(err) => capability_remote_envelope::Message::Result(
                        invocation_error_result(&invocation, err),
                    ),
                };
                Ok(Some(CapabilityRemoteEnvelope { message: Some(msg) }))
            }
            Some(capability_remote_envelope::Message::InvocationFrame(frame)) => {
                let Some(invocation) = frame.invocation else {
                    return Err(CapabilityError::InvalidRequest(
                        "invocation frame must contain an invocation",
                    ));
                };
                let result = self.provider.invoke(
                    &invocation.grant_id,
                    &invocation,
                    Some(&frame.inline_parameters),
                );
                let msg = match result {
                    Ok(resp) => capability_remote_envelope::Message::Result(resp.result),
                    Err(err) => capability_remote_envelope::Message::Result(
                        invocation_error_result(&invocation, err),
                    ),
                };
                Ok(Some(CapabilityRemoteEnvelope { message: Some(msg) }))
            }
            Some(capability_remote_envelope::Message::SessionClose(close)) => {
                self.provider.close_session(&close)?;
                Ok(None)
            }
            Some(capability_remote_envelope::Message::Request(request)) => {
                if let Some(grant) = self.provider.handle_request(&request)? {
                    Ok(Some(CapabilityRemoteEnvelope {
                        message: Some(capability_remote_envelope::Message::Grant(grant)),
                    }))
                } else {
                    Ok(None)
                }
            }
            Some(capability_remote_envelope::Message::Grant(grant)) => {
                self.provider.handle_grant(&grant)?;
                Ok(None)
            }
            Some(capability_remote_envelope::Message::Revocation(revocation)) => {
                self.provider.handle_revocation(&revocation)?;
                Ok(None)
            }
            Some(capability_remote_envelope::Message::SessionAccept(_))
            | Some(capability_remote_envelope::Message::SessionEvent(_))
            | Some(capability_remote_envelope::Message::Result(_))
            | Some(capability_remote_envelope::Message::ResultFrame(_))
            | None => Ok(None),
        }
    }
}

fn invocation_error_result(
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

// ---------------------------------------------------------------------------
