use crate::prelude::v1::*;

pub use crate::crypto::sha256;
pub use crate::result::{
    accept, defer, duplicate, empty_map, reject, ReasonCode, ValidationResult, Verdict,
};
pub use crate::util::{bytes_to_hex, must_hex_to_bytes, parse_rfc3339};
pub use crate::value::{mapping, seq, ystr, Value};
pub use std::collections::{BTreeMap, BTreeSet};

pub trait FixtureVerifier {
    fn verify_signed_fixture(
        &self,
        payload: &BTreeMap<String, Value>,
        expected_fixture: &str,
    ) -> Option<bool>;
}

pub fn parse_ts(s: &str) -> Result<crate::util::DateTimeUtc, ReasonCode> {
    parse_rfc3339(s).map_err(|_| ReasonCode::StructuralInvalid)
}

pub fn get<'a>(m: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a Value> {
    m.get(key)
}

pub fn get_map<'a>(
    m: &'a BTreeMap<String, Value>,
    key: &str,
) -> Option<&'a BTreeMap<String, Value>> {
    get(m, key)?.as_map()
}
pub fn get_seq<'a>(m: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a [Value]> {
    get(m, key)?.as_seq()
}
pub fn string_value(m: &BTreeMap<String, Value>, key: &str, fallback: &str) -> String {
    get(m, key)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}
pub fn number_value(m: &BTreeMap<String, Value>, key: &str, fallback: i64) -> i64 {
    get(m, key).and_then(Value::as_i64).unwrap_or(fallback)
}
pub fn set_from_list(v: Option<&Value>) -> BTreeSet<String> {
    v.and_then(Value::as_seq)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}
pub fn map_value<'a>(v: &'a Value, key: &str) -> Option<&'a BTreeMap<String, Value>> {
    v.as_map()?.get(key)?.as_map()
}

pub fn scope_allows(granted: Option<&Value>, requested: Option<&Value>) -> bool {
    if matches!(requested, None | Some(Value::Null)) {
        return true;
    }
    if matches!(requested, Some(Value::String(s)) if s.is_empty() || s == "*") {
        return true;
    }
    if matches!(granted, None | Some(Value::Null)) {
        return true;
    }
    if matches!(granted, Some(Value::String(s)) if s.is_empty() || s == "*") {
        return true;
    }
    if granted == requested {
        return true;
    }
    let gs = granted.and_then(Value::as_str);
    let rs = requested.and_then(Value::as_str);
    if let (Some(gs), Some(rs)) = (gs, rs) {
        if gs.starts_with("node:")
            || rs.starts_with("node:")
            || (gs.starts_with("timeline:") && rs.starts_with("timeline:"))
        {
            return rs.starts_with(gs);
        }
        return true;
    }
    false
}

pub fn delegation_effective_action_set(chain: &[Value]) -> BTreeSet<String> {
    let mut actions: Option<BTreeSet<String>> = None;
    for item in chain {
        let Some(link) = item.as_map() else {
            continue;
        };
        let current: BTreeSet<String> = get_map(link, "capability")
            .and_then(|cap| get_seq(cap, "actions"))
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default();
        actions = Some(match actions {
            None => current,
            Some(prev) => prev.intersection(&current).cloned().collect(),
        });
    }
    actions.unwrap_or_default()
}

pub fn delegation_effective_scope(chain: &[Value]) -> Result<Option<Value>, ()> {
    let mut scope: Option<Value> = None;
    for item in chain {
        let Some(link) = item.as_map() else {
            continue;
        };
        let candidate = get_map(link, "capability")
            .and_then(|cap| cap.get("scope"))
            .cloned();
        let Some(candidate) = candidate else {
            continue;
        };
        if scope.is_none() {
            scope = Some(candidate);
            continue;
        }
        match (scope.as_ref().and_then(Value::as_str), candidate.as_str()) {
            (Some(a), Some(b)) => {
                if b.starts_with(a) {
                    scope = Some(candidate);
                } else if a.starts_with(b) {
                } else {
                    return Err(());
                }
            }
            _ if scope.as_ref() == Some(&candidate) => {}
            _ => return Err(()),
        }
    }
    Ok(scope)
}

pub fn command_required_action(command: &BTreeMap<String, Value>) -> String {
    for key in ["required_action", "action"] {
        if let Some(v) = command.get(key).and_then(Value::as_str) {
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    match string_value(command, "command_type", "").as_str() {
        "QUERY" => "query".to_string(),
        "ADD_CONTROLLER" | "REMOVE_CONTROLLER" | "TRANSFER_CONTROL" => "node_control".to_string(),
        _ => "command".to_string(),
    }
}

pub fn assurance_rank(class: &str) -> i64 {
    match class {
        "ASSURANCE_CLASS_SOFTWARE" => 1,
        "ASSURANCE_CLASS_HARDWARE_BACKED" => 2,
        "ASSURANCE_CLASS_ATTESTED_RUNTIME" => 3,
        _ => 0,
    }
}

pub fn has_any_query_bound(query: &BTreeMap<String, Value>) -> bool {
    query.contains_key("target_scope")
        || query.contains_key("time_window")
        || query.contains_key("checkpoint_base")
        || query.contains_key("result_limit")
        || query.contains_key("cost_limit")
        || query.contains_key("query_payload_object")
}

pub fn nonce_bytes(value: &Value) -> Option<Vec<u8>> {
    match value {
        Value::String(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}
