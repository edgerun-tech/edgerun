use super::helpers::*;
pub fn validate_event_family_semantics(
    semantic_input: &BTreeMap<String, Value>,
    event: &BTreeMap<String, Value>,
) -> Option<ValidationResult> {
    match string_value(event, "event_type", "").as_str() {
        "EVENT_TYPE_NODE_GENESIS" => {
            let payload = get_map(semantic_input, "node_genesis_payload")?;
            if number_value(event, "seq", -1) != 0 {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            if string_value(payload, "node_id", "").is_empty()
                || !payload.contains_key("primary_node_identity")
                || get_seq(payload, "initial_controllers")
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
            {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        "EVENT_TYPE_COMMAND_SENT" => {
            let payload = get_map(semantic_input, "command_sent_payload")?;
            if !payload.contains_key("command") || !payload.contains_key("target_node") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        "EVENT_TYPE_COMMAND_COMMITTED" | "EVENT_TYPE_COMMAND_REJECTED" => {
            let payload = get_map(semantic_input, "command_result_payload")?;
            if !payload.contains_key("command") || !payload.contains_key("issuer") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            let expected =
                if string_value(event, "event_type", "") == "EVENT_TYPE_COMMAND_COMMITTED" {
                    "COMMAND_DECISION_COMMITTED"
                } else {
                    "COMMAND_DECISION_REJECTED"
                };
            if string_value(payload, "decision", "") != expected {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        "EVENT_TYPE_ACTION_STARTED"
        | "EVENT_TYPE_ACTION_COMPLETED"
        | "EVENT_TYPE_ACTION_FAILED" => {
            let payload = get_map(semantic_input, "action_lifecycle_payload")?;
            if !payload.contains_key("origin_command")
                || string_value(payload, "action_instance_id", "").is_empty()
            {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            let expected = match string_value(event, "event_type", "").as_str() {
                "EVENT_TYPE_ACTION_STARTED" => "ACTION_STATUS_STARTED",
                "EVENT_TYPE_ACTION_COMPLETED" => "ACTION_STATUS_COMPLETED",
                _ => "ACTION_STATUS_FAILED",
            };
            if string_value(payload, "status", "") != expected {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            if expected == "ACTION_STATUS_COMPLETED" && !payload.contains_key("result_object") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            if expected == "ACTION_STATUS_FAILED" && !payload.contains_key("error_object") {
                return Some(reject(
                    ReasonCode::StructuralInvalid,
                    empty_map(),
                    empty_map(),
                ));
            }
            None
        }
        _ => None,
    }
}
