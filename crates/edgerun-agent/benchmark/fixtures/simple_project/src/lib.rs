use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub timeout: u64,
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            timeout: 30,
            verbose: false,
        }
    }
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("name cannot be empty".to_string());
        }
        if self.timeout == 0 {
            return Err("timeout must be > 0".to_string());
        }
        Ok(())
    }
}

pub fn process(input: &str, config: &Config) -> String {
    if config.verbose {
        eprintln!("Processing: {}", input);
    }
    format!("processed: {}", input)
}

pub fn parse_line(line: &str) -> Option<(String, u32)> {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    let key = parts[0].trim().to_string();
    let value: u32 = parts[1].trim().parse().ok()?;
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let c = Config::default();
        assert_eq!(c.name, "default");
        assert_eq!(c.timeout, 30);
    }

    #[test]
    fn test_config_validate_empty_name() {
        let c = Config { name: String::new(), timeout: 1, verbose: false };
        assert!(c.validate().is_err());
    }

    #[test]
    fn test_process() {
        let c = Config::default();
        assert_eq!(process("hello", &c), "processed: hello");
    }

    #[test]
    fn test_parse_line() {
        assert_eq!(parse_line("foo: 42"), Some(("foo".to_string(), 42)));
        assert_eq!(parse_line("no colon here"), None);
    }
}
