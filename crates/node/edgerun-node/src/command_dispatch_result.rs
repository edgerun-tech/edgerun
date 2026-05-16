//! Shared legacy command result helpers.
//!
//! These helpers build references for local audit events around `edgerun-core`
//! command streams. They are not admission, routing, or settlement authority.

use edgerun_protocols::core_protocol::command::command_hash;
use edgerun_protocols::core_protocol::protocol::{CommandEnvelope, CommandRef};

pub fn command_ref_from(command: &CommandEnvelope) -> CommandRef {
    CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    }
}
