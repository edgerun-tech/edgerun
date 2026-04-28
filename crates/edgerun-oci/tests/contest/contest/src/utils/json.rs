use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use edgerun_json::{JsonValue, Map};

use super::test_utils::{LifecycleStatus, State};

pub fn parse(input: impl AsRef<str>) -> Result<JsonValue> {
    edgerun_json::from_str(input.as_ref()).map_err(|error| anyhow!(error.to_string()))
}

pub fn parse_slice(input: &[u8]) -> Result<JsonValue> {
    edgerun_json::from_slice(input).map_err(|error| anyhow!(error.to_string()))
}

pub fn to_vec_pretty(value: impl Borrow<JsonValue>) -> Result<Vec<u8>> {
    edgerun_json::to_vec_pretty(value.borrow()).map_err(|error| anyhow!(error.to_string()))
}

pub fn to_string_pretty(value: impl Borrow<JsonValue>) -> Result<String> {
    edgerun_json::to_string_pretty(value.borrow()).map_err(|error| anyhow!(error.to_string()))
}

pub fn parse_lifecycle_status(value: &JsonValue) -> Result<LifecycleStatus> {
    match value.as_str() {
        Some("creating") => Ok(LifecycleStatus::Creating),
        Some("created") => Ok(LifecycleStatus::Created),
        Some("running") => Ok(LifecycleStatus::Running),
        Some("stopped") => Ok(LifecycleStatus::Stopped),
        Some(other) => bail!("unknown lifecycle status `{other}`"),
        None => bail!("lifecycle status is not a string"),
    }
}

pub fn parse_state(input: &str) -> Result<State> {
    state_from_value(parse(input)?)
}

pub fn state_from_value(value: JsonValue) -> Result<State> {
    let mut object = match value {
        JsonValue::Object(object) => object,
        other => bail!("container state is not an object: {}", type_name(&other)),
    };

    Ok(State {
        oci_version: take_string(&mut object, "ociVersion")?,
        id: take_string(&mut object, "id")?,
        status: take_string(&mut object, "status")?,
        pid: take_optional_i32(&mut object, "pid")?,
        bundle: PathBuf::from(take_string(&mut object, "bundle")?),
        annotations: take_optional_string_map(&mut object, "annotations")?,
        created: take_optional_string(&mut object, "created")?,
        creator: take_optional_u32(&mut object, "creator")?,
        use_systemd: take_optional_bool(&mut object, "useSystemd")?,
    })
}

fn take(object: &mut Map, key: &str) -> Option<JsonValue> {
    object.remove(key).filter(|value| !value.is_null())
}

fn take_string(object: &mut Map, key: &str) -> Result<String> {
    take(object, key)
        .and_then(|value| match value {
            JsonValue::String(value) => Some(value),
            _ => None,
        })
        .ok_or_else(|| anyhow!("missing or invalid string field `{key}`"))
}

fn take_optional_string(object: &mut Map, key: &str) -> Result<Option<String>> {
    match take(object, key) {
        Some(JsonValue::String(value)) => Ok(Some(value)),
        Some(other) => bail!("field `{key}` expected string, got {}", type_name(&other)),
        None => Ok(None),
    }
}

fn type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "bool",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

fn take_optional_i32(object: &mut Map, key: &str) -> Result<Option<i32>> {
    match take(object, key) {
        Some(value) => Ok(Some(
            value
                .as_i32()
                .ok_or_else(|| anyhow!("field `{key}` expected i32"))?,
        )),
        None => Ok(None),
    }
}

fn take_optional_u32(object: &mut Map, key: &str) -> Result<Option<u32>> {
    match take(object, key) {
        Some(value) => Ok(Some(
            value
                .as_u32()
                .ok_or_else(|| anyhow!("field `{key}` expected u32"))?,
        )),
        None => Ok(None),
    }
}

fn take_optional_bool(object: &mut Map, key: &str) -> Result<Option<bool>> {
    match take(object, key) {
        Some(value) => Ok(Some(
            value
                .as_bool()
                .ok_or_else(|| anyhow!("field `{key}` expected bool"))?,
        )),
        None => Ok(None),
    }
}

fn take_optional_string_map(
    object: &mut Map,
    key: &str,
) -> Result<Option<HashMap<String, String>>> {
    let Some(value) = take(object, key) else {
        return Ok(None);
    };
    let JsonValue::Object(map) = value else {
        bail!("field `{key}` expected object");
    };
    let mut out = HashMap::new();
    for (key, value) in map.into_vec() {
        let JsonValue::String(value) = value else {
            bail!("annotation `{key}` expected string");
        };
        out.insert(key, value);
    }
    Ok(Some(out))
}
