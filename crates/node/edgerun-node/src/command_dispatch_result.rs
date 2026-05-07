//! Shared command result helpers.

use edgerun_protocols::core_protocol::command::command_hash;
use edgerun_protocols::core_protocol::protocol::{CommandEnvelope, CommandRef};

pub fn command_ref_from(command: &CommandEnvelope) -> CommandRef {
    CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    }
}
