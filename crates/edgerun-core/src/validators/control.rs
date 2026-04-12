use super::helpers::*;
pub fn validate_control_change_case(
    semantic_input: &BTreeMap<String, Value>,
    local_state: &BTreeMap<String, Value>,
    verifier: &dyn FixtureVerifier,
) -> ValidationResult {
    let Some(command) = get_map(semantic_input, "command") else {
        return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
    };
    let expected = command
        .get("signature_fixture")
        .and_then(Value::as_str)
        .unwrap_or(&string_value(command, "issuer", ""))
        .to_string();
    if matches!(
        verifier.verify_signed_fixture(command, &expected),
        Some(false)
    ) {
        return reject(ReasonCode::CryptoInvalid, empty_map(), empty_map());
    }
    let mut controllers = set_from_list(local_state.get("current_controller_set"));
    let min_controllers = get_map(local_state, "control_policy")
        .map(|m| number_value(m, "minimum_controllers", 1))
        .unwrap_or(1) as usize;
    match string_value(command, "command_type", "").as_str() {
        "ADD_CONTROLLER" => {
            let new = string_value(command, "new_controller", "");
            if new.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            controllers.insert(new);
            accept(
                mapping([
                    ("decision", ystr("committed")),
                    ("control_delta", ystr("add_controller")),
                ]),
                mapping([(
                    "controller_set",
                    seq(controllers.into_iter().map(Value::String)),
                )]),
            )
        }
        "REMOVE_CONTROLLER" => {
            let target = string_value(command, "target_controller", "");
            if target.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            controllers.remove(&target);
            if controllers.len() < min_controllers {
                return reject(ReasonCode::ControlInvariantFailed, empty_map(), empty_map());
            }
            accept(
                mapping([
                    ("decision", ystr("committed")),
                    ("control_delta", ystr("remove_controller")),
                ]),
                mapping([(
                    "controller_set",
                    seq(controllers.into_iter().map(Value::String)),
                )]),
            )
        }
        "TRANSFER_CONTROL" => {
            let from = string_value(command, "from", "");
            let to = string_value(command, "to", "");
            if from.is_empty() || to.is_empty() {
                return reject(ReasonCode::StructuralInvalid, empty_map(), empty_map());
            }
            let valid = get_map(local_state, "proof_of_possession")
                .and_then(|m| m.get(&to))
                .and_then(Value::as_str)
                == Some("valid-challenge-response");
            if !valid {
                return defer(ReasonCode::MissingDependency, empty_map());
            }
            controllers.remove(&from);
            controllers.insert(to);
            if controllers.len() < min_controllers {
                return reject(ReasonCode::ControlInvariantFailed, empty_map(), empty_map());
            }
            accept(
                mapping([
                    ("decision", ystr("committed")),
                    ("control_delta", ystr("transfer_control")),
                ]),
                mapping([(
                    "controller_set",
                    seq(controllers.into_iter().map(Value::String)),
                )]),
            )
        }
        _ => reject(ReasonCode::StructuralInvalid, empty_map(), empty_map()),
    }
}
