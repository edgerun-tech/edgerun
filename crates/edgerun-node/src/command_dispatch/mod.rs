//! Command dispatch for the edgerun node daemon.
//!
//! Replaces the old signature-only validation with:
//! - Full command validation (replay, timing, delegation signature verification)
//! - Command type dispatch (ADD_CONTROLLER, REMOVE_CONTROLLER, TRANSFER_CONTROL, etc.)
//! - Controller set management and projection from event log
//! - Revocation record processing

mod controller_set;
mod result;
mod signature;
mod project;
mod dispatch_main;
mod controllers;
mod workload;
mod delegation;
mod events;

pub use controller_set::ControllerSet;
pub use result::CommandDispatchResult;
pub use dispatch_main::dispatch_command;
pub use project::project_controller_set;
pub use events::{
    create_node_genesis_payload,
    record_command_sent_event,
    record_action_event,
};
pub(crate) use events::{
    append_signed_event,
    sign_event_envelope,
};

#[cfg(test)]
mod tests;
