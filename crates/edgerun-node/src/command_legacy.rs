//! Explicit boundary around the legacy command dispatcher.
//!
//! New routing code should depend on this module instead of reaching into
//! `command_dispatch.rs` directly. As handlers are extracted, this module should
//! shrink until it can be deleted.

pub use crate::command_dispatch::{
    create_node_genesis_payload, dispatch_command, project_config,
    project_config_from_base, project_controller_set, record_command_sent_event,
    sign_and_append_event, sign_and_append_event_blocking, sign_event_envelope,
    CommandDispatchResult, ControllerSet,
};
