//! Shared command result helpers.

use edgerun_core::command::command_hash;
use edgerun_core::protocol::{CommandEnvelope, CommandRef};

pub fn command_ref_from(command: &CommandEnvelope) -> CommandRef {
    CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    }
}
