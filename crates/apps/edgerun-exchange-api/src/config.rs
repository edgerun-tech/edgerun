//! Exchange API configuration.

extern crate alloc;

pub struct Config {
    pub port: u16,
    pub host: alloc::string::String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".into(),
        }
    }
}
