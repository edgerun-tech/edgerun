//! Server resource command dispatch planning and execution.
//!
//! This module prepares and commits typed server resource commands so the
//! legacy dispatcher can later delegate to it with a small router splice.

use crate::command_dispatch_event::{append_command_result_event, CommandResultEventWrite};
use crate::command_dispatch_payload::inline_payload_bytes;
use crate::server_resources::{
    apply_server_resource_event, compile_server_plan, decode_command_payload,
    encode_committed_resource_event, project_server_resources, DerivedServerPlan,
    ServerResourceEvent,
};
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::common::ObjectRef;
use edgerun_proto::edgerun::v0::stream::CommandEnvelope;
use edgerun_storage::NodeStore;

#[derive(Clone, Debug)]
pub struct ServerResourceDispatchPlan {
    pub resource_event: ServerResourceEvent,
    pub result_object: ObjectRef,
    pub desired_plan: DerivedServerPlan,
}

pub fn plan_server_resource_command(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
) -> Result<ServerResourceDispatchPlan, String> {
    let payload = inline_payload_bytes(command)
        .ok_or_else(|| "missing_server_resource_payload".to_string())?;
    let resource_event = decode_command_payload(command.command_type, payload)?;

    let mut projection = project_server_resources(store, stream_id)?;
    apply_server_resource_event(&mut projection, resource_event.clone())?;

    let desired_plan = compile_server_plan(&projection, &Default::default());
    let committed = encode_committed_resource_event(&resource_event, None);
    let result_object = store
        .put_object(
            &committed,
            edgerun_proto::edgerun::v0::common::ObjectKind::DerivedView as i32,
            &[stream_id.to_vec()],
        )
        .map_err(|e| format!("storage_failed: {e}"))?;

    Ok(ServerResourceDispatchPlan {
        resource_event,
        result_object,
        desired_plan,
    })
}

pub fn dispatch_server_resource_command(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
) -> Result<(ServerResourceDispatchPlan, CommandResultEventWrite), String> {
    let plan = plan_server_resource_command(command, store, stream_id)?;
    let event_write = append_command_result_event(
        command,
        store,
        stream_id,
        signer,
        true,
        "",
        Some(plan.result_object.clone()),
    )?;
    Ok((plan, event_write))
}

pub fn reject_server_resource_command(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    reason_code: &str,
) -> Result<CommandResultEventWrite, String> {
    append_command_result_event(command, store, stream_id, signer, false, reason_code, None)
}
