//! Bridge from the legacy command dispatcher shape to typed server resources.
//!
//! This lets `command_dispatch.rs` delegate server-resource commands with a
//! single router branch while the old dispatcher is being decomposed.

use crate::command_dispatch::CommandDispatchResult;
use crate::server_resource_dispatch::{
    dispatch_server_resource_command, reject_server_resource_command,
};
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, EventType};
use edgerun_storage::NodeStore;

pub fn dispatch_server_resource_command_result(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
) -> CommandDispatchResult {
    match dispatch_server_resource_command(command, store, stream_id, signer) {
        Ok((_plan, event_write)) => CommandDispatchResult {
            event_type: event_write.event_type,
            decision: CommandDecision::Committed as i32,
            reason_code: String::new(),
            response_bytes: event_write.response_bytes,
        },
        Err(reason) => {
            match reject_server_resource_command(command, store, stream_id, signer, &reason) {
                Ok(event_write) => CommandDispatchResult {
                    event_type: event_write.event_type,
                    decision: CommandDecision::Rejected as i32,
                    reason_code: reason,
                    response_bytes: event_write.response_bytes,
                },
                Err(storage_reason) => CommandDispatchResult {
                    event_type: EventType::CommandRejected,
                    decision: CommandDecision::Rejected as i32,
                    reason_code: storage_reason,
                    response_bytes: Vec::new(),
                },
            }
        }
    }
}
