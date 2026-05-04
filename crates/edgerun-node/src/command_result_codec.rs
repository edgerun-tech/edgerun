//! Command-result payload codec boundary.
//!
//! Node dispatch still receives protobuf boundary structs, but committed command
//! result payload bytes should be deterministic edgerun-wire bytes. Keep this
//! helper small so command dispatch does not call prost encoding directly.

use edgerun_proto::edgerun::v0::stream::CommandResultPayload;

#[must_use]
pub fn encode_command_result_payload(payload: &CommandResultPayload) -> Vec<u8> {
    edgerun_core::wire_command::command_result_bytes(payload)
}
