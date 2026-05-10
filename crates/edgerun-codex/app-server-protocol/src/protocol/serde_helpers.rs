use edgerun_serde::Deserialize;
use edgerun_serde::Deserializer;
use edgerun_serde::Serialize;
use edgerun_serde::Serializer;

pub fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    edgerun_serde_with::rust::double_option::deserialize(deserializer)
}

pub fn serialize_double_option<T, S>(
    value: &Option<Option<T>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    edgerun_serde_with::rust::double_option::serialize(value, serializer)
}
