use crate::prelude::v1::*;
use edgerun_capabilities::CapabilityError;
use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityInvocation, CapabilityResult,
};

use crate::protocol::RemoteInvocationResult;

pub(super) fn stream_oriented_error(
    invocation: &CapabilityInvocation,
    label: &str,
) -> RemoteInvocationResult {
    RemoteInvocationResult {
        result: CapabilityResult {
            result_version: 1,
            invocation_id: invocation.invocation_id.clone(),
            grant_id: invocation.grant_id.clone(),
            success: false,
            result_access_class: invocation.requested_access_class,
            produced_event_kinds: Vec::new(),
            payload_object: None,
            error_reason: format!("{label} remote adapter is stream-oriented; use session events"),
            produced_at: None,
            signature: None,
        },
        inline_payload: Vec::new(),
    }
}
