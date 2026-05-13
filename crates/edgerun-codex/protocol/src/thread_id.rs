use std::fmt::Display;

use crate::local_uuid;
use crate::local_uuid::Uuid;
use edgerun_serde::Deserialize;
use edgerun_serde::Serialize;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TS, Hash)]
#[ts(type = "string")]
pub struct ThreadId {
    pub(crate) uuid: Uuid,
}

impl ThreadId {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::now_v7(),
        }
    }

    pub fn from_string(s: &str) -> Result<Self, local_uuid::Error> {
        Ok(Self {
            uuid: Uuid::parse_str(s)?,
        })
    }
}

impl TryFrom<&str> for ThreadId {
    type Error = local_uuid::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_string(value)
    }
}

impl TryFrom<String> for ThreadId {
    type Error = local_uuid::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_string(value.as_str())
    }
}

impl From<ThreadId> for String {
    fn from(value: ThreadId) -> Self {
        value.to_string()
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.uuid, f)
    }
}

impl Serialize for ThreadId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: edgerun_serde::Serializer,
    {
        serializer.collect_str(&self.uuid)
    }
}

impl<'de> Deserialize<'de> for ThreadId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: edgerun_serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let uuid = Uuid::parse_str(&value).map_err(edgerun_serde::de::Error::custom)?;
        Ok(Self { uuid })
    }
}

impl JsonSchema for ThreadId {
    fn schema_name() -> String {
        "ThreadId".to_string()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        <String>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_thread_id_default_is_not_zeroes() {
        let id = ThreadId::default();
        assert_ne!(id.uuid, Uuid::nil());
    }
}
