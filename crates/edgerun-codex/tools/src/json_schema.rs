use edgerun_json::Value as JsonValue;
use edgerun_json::json;
use edgerun_json::{FromJson, JsonValueError, Map, ToJson};
use std::collections::BTreeMap;

/// Primitive JSON Schema type names we support in tool definitions.
///
/// This mirrors the OpenAI Structured Outputs subset for JSON Schema `type`:
/// string, number, boolean, integer, object, array, and null.
/// Keywords such as `enum`, `const`, and `anyOf` are modeled separately.
/// See <https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonSchemaPrimitiveType {
    String,
    Number,
    Boolean,
    Integer,
    Object,
    Array,
    Null,
}

impl ToJson for JsonSchemaPrimitiveType {
    fn to_json(&self) -> JsonValue {
        JsonValue::String(schema_type_name(*self).to_string())
    }
}

impl FromJson for JsonSchemaPrimitiveType {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let value = String::from_json(value)?;
        schema_type_from_str(&value)
            .ok_or_else(|| JsonValueError::WrongType(format!("unknown JSON Schema type `{value}`")))
    }
}

/// JSON Schema `type` supports either a single type name or a union of names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonSchemaType {
    Single(JsonSchemaPrimitiveType),
    Multiple(Vec<JsonSchemaPrimitiveType>),
}

impl ToJson for JsonSchemaType {
    fn to_json(&self) -> JsonValue {
        match self {
            Self::Single(schema_type) => schema_type.to_json(),
            Self::Multiple(schema_types) => schema_types.to_json(),
        }
    }
}

impl FromJson for JsonSchemaType {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match value {
            JsonValue::String(_) => JsonSchemaPrimitiveType::from_json(value).map(Self::Single),
            JsonValue::Array(_) => {
                Vec::<JsonSchemaPrimitiveType>::from_json(value).map(Self::Multiple)
            }
            other => Err(JsonValueError::WrongType(format!(
                "JSON Schema type expected string or array, found {}",
                other.variant_name()
            ))),
        }
    }
}

/// Generic JSON-Schema subset needed for our tool definitions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsonSchema {
    pub schema_type: Option<JsonSchemaType>,
    pub description: Option<String>,
    pub enum_values: Option<Vec<JsonValue>>,
    pub items: Option<Box<JsonSchema>>,
    pub properties: Option<BTreeMap<String, JsonSchema>>,
    pub required: Option<Vec<String>>,
    pub additional_properties: Option<AdditionalProperties>,
    pub any_of: Option<Vec<JsonSchema>>,
}

impl JsonSchema {
    /// Construct a scalar/object/array schema with a single JSON Schema type.
    fn typed(schema_type: JsonSchemaPrimitiveType, description: Option<String>) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(schema_type)),
            description,
            ..Default::default()
        }
    }

    pub fn any_of(variants: Vec<JsonSchema>, description: Option<String>) -> Self {
        Self {
            description,
            any_of: Some(variants),
            ..Default::default()
        }
    }

    pub fn boolean(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Boolean, description)
    }

    pub fn string(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::String, description)
    }

    pub fn number(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Number, description)
    }

    pub fn integer(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Integer, description)
    }

    pub fn null(description: Option<String>) -> Self {
        Self::typed(JsonSchemaPrimitiveType::Null, description)
    }

    pub fn string_enum(values: Vec<JsonValue>, description: Option<String>) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::String)),
            description,
            enum_values: Some(values),
            ..Default::default()
        }
    }

    pub fn array(items: JsonSchema, description: Option<String>) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Array)),
            description,
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    pub fn object(
        properties: BTreeMap<String, JsonSchema>,
        required: Option<Vec<String>>,
        additional_properties: Option<AdditionalProperties>,
    ) -> Self {
        Self {
            schema_type: Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Object)),
            properties: Some(properties),
            required,
            additional_properties,
            ..Default::default()
        }
    }
}

impl ToJson for JsonSchema {
    fn to_json(&self) -> JsonValue {
        let mut object = Map::new();
        object.push_opt_field("type", self.schema_type.as_ref().map(ToJson::to_json));
        object.push_opt_field(
            "description",
            self.description.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field("enum", self.enum_values.as_ref().map(ToJson::to_json));
        object.push_opt_field("items", self.items.as_ref().map(ToJson::to_json));
        object.push_opt_field("properties", self.properties.as_ref().map(ToJson::to_json));
        object.push_opt_field("required", self.required.as_ref().map(ToJson::to_json));
        object.push_opt_field(
            "additionalProperties",
            self.additional_properties.as_ref().map(ToJson::to_json),
        );
        object.push_opt_field("anyOf", self.any_of.as_ref().map(ToJson::to_json));
        object.into()
    }
}

impl FromJson for JsonSchema {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("JsonSchema")?;
        Ok(Self {
            schema_type: object.take_optional("type")?,
            description: object.take_optional("description")?,
            enum_values: object.take_optional("enum")?,
            items: object.take_optional("items")?,
            properties: object.take_optional("properties")?,
            required: object.take_optional("required")?,
            additional_properties: object.take_optional("additionalProperties")?,
            any_of: object.take_optional("anyOf")?,
        })
    }
}

/// Whether additional properties are allowed, and if so, any required schema.
#[derive(Debug, Clone, PartialEq)]
pub enum AdditionalProperties {
    Boolean(bool),
    Schema(Box<JsonSchema>),
}

impl ToJson for AdditionalProperties {
    fn to_json(&self) -> JsonValue {
        match self {
            Self::Boolean(value) => value.to_json(),
            Self::Schema(schema) => schema.to_json(),
        }
    }
}

impl FromJson for AdditionalProperties {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match value {
            JsonValue::Bool(value) => Ok(Self::Boolean(value)),
            JsonValue::Object(_) => {
                JsonSchema::from_json(value).map(|schema| Self::Schema(Box::new(schema)))
            }
            other => Err(JsonValueError::WrongType(format!(
                "additionalProperties expected boolean or schema object, found {}",
                other.variant_name()
            ))),
        }
    }
}

impl From<bool> for AdditionalProperties {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<JsonSchema> for AdditionalProperties {
    fn from(value: JsonSchema) -> Self {
        Self::Schema(Box::new(value))
    }
}

/// Parse the tool `input_schema` or return an error for invalid schema.
pub fn parse_tool_input_schema(
    input_schema: &JsonValue,
) -> Result<JsonSchema, edgerun_json::Error> {
    let mut input_schema = input_schema.clone();
    sanitize_json_schema(&mut input_schema);
    let schema: JsonSchema = edgerun_json::from_json_value(input_schema)?;
    if matches!(
        schema.schema_type,
        Some(JsonSchemaType::Single(JsonSchemaPrimitiveType::Null))
    ) {
        return Err(singleton_null_schema_error());
    }
    Ok(schema)
}

/// Sanitize a JSON Schema (as edgerun_json::Value) so it can fit our limited
/// schema representation. This function:
/// - Ensures every typed schema object has a `"type"` when required.
/// - Preserves explicit `anyOf`.
/// - Collapses `const` into single-value `enum`.
/// - Fills required child fields for object/array schema types, including
///   nullable unions, with permissive defaults when absent.
fn sanitize_json_schema(value: &mut JsonValue) {
    match value {
        JsonValue::Bool(_) => {
            // JSON Schema boolean form: true/false. Coerce to an accept-all string.
            *value = json!({ "type": "string" });
        }
        JsonValue::Array(values) => {
            for value in values {
                sanitize_json_schema(value);
            }
        }
        JsonValue::Object(map) => {
            if let Some(properties) = map.get_mut("properties")
                && let Some(properties_map) = properties.as_object_mut()
            {
                for value in properties_map.values_mut() {
                    sanitize_json_schema(value);
                }
            }
            if let Some(items) = map.get_mut("items") {
                sanitize_json_schema(items);
            }
            if let Some(additional_properties) = map.get_mut("additionalProperties")
                && !matches!(additional_properties, JsonValue::Bool(_))
            {
                sanitize_json_schema(additional_properties);
            }
            if let Some(value) = map.get_mut("prefixItems") {
                sanitize_json_schema(value);
            }
            if let Some(value) = map.get_mut("anyOf") {
                sanitize_json_schema(value);
            }

            if let Some(const_value) = map.remove("const") {
                map.insert("enum".to_string(), JsonValue::Array(vec![const_value]));
            }

            let mut schema_types = normalized_schema_types(map);

            if schema_types.is_empty() && map.contains_key("anyOf") {
                return;
            }

            if schema_types.is_empty() {
                if map.contains_key("properties")
                    || map.contains_key("required")
                    || map.contains_key("additionalProperties")
                {
                    schema_types.push(JsonSchemaPrimitiveType::Object);
                } else if map.contains_key("items") || map.contains_key("prefixItems") {
                    schema_types.push(JsonSchemaPrimitiveType::Array);
                } else if map.contains_key("enum") || map.contains_key("format") {
                    schema_types.push(JsonSchemaPrimitiveType::String);
                } else if map.contains_key("minimum")
                    || map.contains_key("maximum")
                    || map.contains_key("exclusiveMinimum")
                    || map.contains_key("exclusiveMaximum")
                    || map.contains_key("multipleOf")
                {
                    schema_types.push(JsonSchemaPrimitiveType::Number);
                } else {
                    schema_types.push(JsonSchemaPrimitiveType::String);
                }
            }

            write_schema_types(map, &schema_types);
            ensure_default_children_for_schema_types(map, &schema_types);
        }
        _ => {}
    }
}

fn ensure_default_children_for_schema_types(
    map: &mut edgerun_json::Map<String, JsonValue>,
    schema_types: &[JsonSchemaPrimitiveType],
) {
    if schema_types.contains(&JsonSchemaPrimitiveType::Object) && !map.contains_key("properties") {
        map.insert(
            "properties".to_string(),
            JsonValue::Object(edgerun_json::Map::new()),
        );
    }

    if schema_types.contains(&JsonSchemaPrimitiveType::Array) && !map.contains_key("items") {
        map.insert("items".to_string(), json!({ "type": "string" }));
    }
}

fn normalized_schema_types(
    map: &edgerun_json::Map<String, JsonValue>,
) -> Vec<JsonSchemaPrimitiveType> {
    let Some(schema_type) = map.get("type") else {
        return Vec::new();
    };

    match schema_type {
        JsonValue::String(schema_type) => schema_type_from_str(schema_type).into_iter().collect(),
        JsonValue::Array(schema_types) => schema_types
            .iter()
            .filter_map(JsonValue::as_str)
            .filter_map(schema_type_from_str)
            .collect(),
        _ => Vec::new(),
    }
}

fn write_schema_types(
    map: &mut edgerun_json::Map<String, JsonValue>,
    schema_types: &[JsonSchemaPrimitiveType],
) {
    match schema_types {
        [] => {
            map.remove("type");
        }
        [schema_type] => {
            map.insert(
                "type".to_string(),
                JsonValue::String(schema_type_name(*schema_type).to_string()),
            );
        }
        _ => {
            map.insert(
                "type".to_string(),
                JsonValue::Array(
                    schema_types
                        .iter()
                        .map(|schema_type| {
                            JsonValue::String(schema_type_name(*schema_type).to_string())
                        })
                        .collect(),
                ),
            );
        }
    }
}

fn schema_type_from_str(schema_type: &str) -> Option<JsonSchemaPrimitiveType> {
    match schema_type {
        "string" => Some(JsonSchemaPrimitiveType::String),
        "number" => Some(JsonSchemaPrimitiveType::Number),
        "boolean" => Some(JsonSchemaPrimitiveType::Boolean),
        "integer" => Some(JsonSchemaPrimitiveType::Integer),
        "object" => Some(JsonSchemaPrimitiveType::Object),
        "array" => Some(JsonSchemaPrimitiveType::Array),
        "null" => Some(JsonSchemaPrimitiveType::Null),
        _ => None,
    }
}

fn schema_type_name(schema_type: JsonSchemaPrimitiveType) -> &'static str {
    match schema_type {
        JsonSchemaPrimitiveType::String => "string",
        JsonSchemaPrimitiveType::Number => "number",
        JsonSchemaPrimitiveType::Boolean => "boolean",
        JsonSchemaPrimitiveType::Integer => "integer",
        JsonSchemaPrimitiveType::Object => "object",
        JsonSchemaPrimitiveType::Array => "array",
        JsonSchemaPrimitiveType::Null => "null",
    }
}

fn singleton_null_schema_error() -> edgerun_json::Error {
    edgerun_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "tool input schema must not be a singleton null type",
    ))
}

#[cfg(test)]
#[path = "json_schema_tests.rs"]
mod tests;
