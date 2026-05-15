//! TOML value parsing and serialization.

#[cfg(feature = "toml")]
pub use crate::toml_api::{
    TomlError, TomlValue, from_toml_str, json_to_toml, parse_toml_value, to_toml_string,
    toml_to_json,
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
    fn parses_basic_double_quoted_toml_string() {
        let value = from_toml_str("name = \"test\"").unwrap();
        assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(toml_to_json(value)["name"].as_str(), Some("test"));
    }

    #[test]
    fn parses_toml_array() {
        let value = from_toml_str("items = ['a', 'b', 3]").unwrap();
        assert_eq!(
            value
                .get("items")
                .and_then(|v| v.as_array())
                .map(|a| a.len()),
            Some(3)
        );
    }

    #[test]
    fn parses_toml_table() {
        let value = from_toml_str("[server]\nhost = 'localhost'\nport = 8080").unwrap();
        assert_eq!(
            value
                .get("server")
                .and_then(|v| v.get("host"))
                .and_then(|v| v.as_str()),
            Some("localhost")
        );
        assert_eq!(
            value
                .get("server")
                .and_then(|v| v.get("port"))
                .and_then(|v| v.as_i64()),
            Some(8080)
        );
    }
}
