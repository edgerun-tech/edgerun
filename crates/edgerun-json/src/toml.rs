//! TOML parsing/serialization - compatible with toml crate.
//!
//! This module provides drop-in replacements for toml functionality.

#[cfg(feature = "toml")]
pub use crate::toml_api::{
    from_toml_str, parse_toml_value, to_toml_string, toml_to_json, json_to_toml,
    TomlError, TomlValue,
};

#[cfg(all(feature = "toml", feature = "serde"))]
pub use crate::toml_api::{
    from_toml_str_typed, to_toml_string_typed,
};

#[cfg(feature = "toml")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_toml() {
        let value = from_toml_str("name = 'test'\nvalue = 123").unwrap();
        assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(value.get("value").and_then(|v| v.as_i64()), Some(123));
    }

    #[test]
    fn parses_toml_array() {
        let value = from_toml_str("items = ['a', 'b', 3]").unwrap();
        assert_eq!(value.get("items").and_then(|v| v.as_array()).map(|a| a.len()), Some(3));
    }

    #[test]
    fn parses_toml_table() {
        let value = from_toml_str("[server]\nhost = 'localhost'\nport = 8080").unwrap();
        assert_eq!(value.get("server").and_then(|v| v.get("host")).and_then(|v| v.as_str()), Some("localhost"));
        assert_eq!(value.get("server").and_then(|v| v.get("port")).and_then(|v| v.as_i64()), Some(8080));
    }
}