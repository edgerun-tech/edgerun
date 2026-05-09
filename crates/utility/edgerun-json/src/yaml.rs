//! YAML parsing and serialization.

#[cfg(feature = "yaml")]
pub use crate::yaml_api::{
    YamlDeserializer, YamlError, YamlValue, from_yaml_str, json_to_yaml, parse_yaml_value,
    to_yaml_string, yaml_to_json,
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
    fn parses_yaml_array() {
        let value = from_yaml_str("- item1\n- item2\n- 3").unwrap();
        assert_eq!(value[0].as_str(), Some("item1"));
        assert_eq!(value[1].as_str(), Some("item2"));
        assert_eq!(value[2].as_i64(), Some(3));
    }

    #[test]
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
