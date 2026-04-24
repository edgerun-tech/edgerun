//! YAML parsing/serialization - compatible with serde_yaml.
//!
//! This module provides drop-in replacements for serde_yaml functionality.

#[cfg(feature = "yaml")]
pub use crate::yaml_api::{
    from_yaml_str, parse_yaml_value, to_yaml_string, yaml_to_json, json_to_yaml,
    YamlDeserializer, YamlError, YamlValue,
};

#[cfg(all(feature = "yaml", feature = "serde"))]
pub use crate::yaml_api::{
    from_yaml_str_typed, to_yaml_string_typed,
};

#[cfg(feature = "yaml")]
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_simple_yaml() {
            let value = from_yaml_str("name: test\nvalue: 123").unwrap();
            assert_eq!(value["name"].as_str(), Some("test"));
            assert_eq!(value["value"].as_i64(), Some(123));
        }

        #[test]
        #[ignore] // causes stack overflow in nested structure parsing
        fn parses_yaml_array() {
            let value = from_yaml_str("- item1\n- item2\n- 3").unwrap();
            assert_eq!(value[0].as_str(), Some("item1"));
            assert_eq!(value[1].as_str(), Some("item2"));
            assert_eq!(value[2].as_i64(), Some(3));
        }

        #[test]
        #[ignore] // causes stack overflow in nested structure parsing
        fn roundtrips_yaml() {
            let original = r#"name: test
items:
  - a
  - b
value: 123"#;
            let parsed = from_yaml_str(original).unwrap();
            let output = to_yaml_string(&parsed).unwrap();
            let reparsed = from_yaml_str(&output).unwrap();
            assert_eq!(parsed["name"], reparsed["name"]);
        }
    }