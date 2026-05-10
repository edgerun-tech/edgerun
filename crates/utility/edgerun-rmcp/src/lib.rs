//! Edgerun-owned compatibility surface for RMCP model APIs used by Codex.

pub mod model;

pub use model::{ErrorData, ErrorData as RmcpError};

#[cfg(feature = "schemars")]
pub use schemars;

pub use serde;
pub use edgerun_json::serde_json;

#[cfg(feature = "schemars")]
pub(crate) fn serde_json_value_via_edgerun_tape<T: serde::Serialize + ?Sized>(
    value: &T,
) -> Result<serde_json::Value, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let input = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let tape = edgerun_json::parse_json_tape(input).map_err(|error| error.to_string())?;
    let value = tape
        .root(input)
        .and_then(|root| root.to_json_value())
        .ok_or_else(|| "edgerun-json tape did not produce a root value".to_string())?;
    edgerun_json_value_to_serde_json(value)
}

#[cfg(feature = "schemars")]
fn edgerun_json_value_to_serde_json(
    value: edgerun_json::JsonValue,
) -> Result<serde_json::Value, String> {
    match value {
        edgerun_json::JsonValue::Null => Ok(serde_json::Value::Null),
        edgerun_json::JsonValue::Bool(value) => Ok(serde_json::Value::Bool(value)),
        edgerun_json::JsonValue::Number(number) => {
            if let Some(value) = number.as_i64() {
                Ok(serde_json::Value::Number(serde_json::Number::from(value)))
            } else if let Some(value) = number.as_u64() {
                Ok(serde_json::Value::Number(serde_json::Number::from(value)))
            } else {
                let value = number
                    .as_f64()
                    .and_then(serde_json::Number::from_f64)
                    .ok_or_else(|| "invalid floating point JSON number".to_string())?;
                Ok(serde_json::Value::Number(value))
            }
        }
        edgerun_json::JsonValue::String(value) => Ok(serde_json::Value::String(value)),
        edgerun_json::JsonValue::Array(values) => values
            .into_iter()
            .map(edgerun_json_value_to_serde_json)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array),
        edgerun_json::JsonValue::Object(object) => object
            .into_vec()
            .into_iter()
            .map(|(key, value)| edgerun_json_value_to_serde_json(value).map(|value| (key, value)))
            .collect::<Result<serde_json::Map<_, _>, _>>()
            .map(serde_json::Value::Object),
    }
}

pub mod handler {
    pub mod server {
        pub mod tool {
            use std::any::TypeId;
                use std::collections::HashMap;
                use std::sync::Arc;

                use crate::model::JsonObject;
                use crate::serde_json;

            #[cfg(feature = "schemars")]
            pub fn schema_for_type<T: schemars::JsonSchema + std::any::Any>() -> Arc<JsonObject> {
                thread_local! {
                    static CACHE: std::sync::RwLock<HashMap<TypeId, Arc<JsonObject>>> = Default::default();
                }
                CACHE.with(|cache| {
                    if let Some(schema) = cache
                        .read()
                        .expect("schema cache lock poisoned")
                        .get(&TypeId::of::<T>())
                    {
                        return schema.clone();
                    }
                    let schema = schemars::schema_for!(T);
                    let value = crate::serde_json_value_via_edgerun_tape(&schema)
                        .expect("failed to convert schema through edgerun-json tape");
                    let object = crate::model::object(value);
                    let schema = Arc::new(object);
                    cache
                        .write()
                        .expect("schema cache lock poisoned")
                        .insert(TypeId::of::<T>(), schema.clone());
                    schema
                })
            }

            #[cfg(feature = "schemars")]
            pub fn schema_for_output<T: schemars::JsonSchema + std::any::Any>()
            -> Result<Arc<JsonObject>, String> {
                let schema = schema_for_type::<T>();
                match schema.get("type") {
                    Some(serde_json::Value::String(kind)) if kind == "object" => Ok(schema),
                    Some(serde_json::Value::String(kind)) => Err(format!(
                        "MCP specification requires tool outputSchema to have root type 'object', but found '{kind}'."
                    )),
                    None => Err(
                        "Schema is missing 'type' field. MCP specification requires outputSchema to have root type 'object'.".to_string()
                    ),
                    Some(other) => Err(format!(
                        "Schema 'type' field has unexpected format: {other:?}. Expected \"object\"."
                    )),
                }
            }
        }
    }
}
