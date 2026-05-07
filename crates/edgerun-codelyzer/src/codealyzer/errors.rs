use std::path::PathBuf;

use edgerun_json::{JsonValue, ToJson};

#[derive(Debug, Clone)]
pub enum AnalyzerError {
    CrateNotFound(String),
    ParseError { file: PathBuf, message: String },
    IoError(String),
    NotImplemented(String),
    VisibilityViolation(String),
}

impl ToJson for AnalyzerError {
    fn to_json(&self) -> JsonValue {
        match self {
            AnalyzerError::CrateNotFound(name) => {
                let mut map = edgerun_json::Map::new();
                map.insert("CrateNotFound".into(), name.to_json());
                JsonValue::Object(map)
            }
            AnalyzerError::ParseError { file, message } => {
                let mut map = edgerun_json::Map::new();
                map.insert("file".into(), file.to_string_lossy().into_owned().to_json());
                map.insert("message".into(), message.to_json());
                JsonValue::Object(map)
            }
            AnalyzerError::IoError(msg) => {
                let mut map = edgerun_json::Map::new();
                map.insert("IoError".into(), msg.to_json());
                JsonValue::Object(map)
            }
            AnalyzerError::NotImplemented(feature) => {
                let mut map = edgerun_json::Map::new();
                map.insert("NotImplemented".into(), feature.to_json());
                JsonValue::Object(map)
            }
            AnalyzerError::VisibilityViolation(msg) => {
                let mut map = edgerun_json::Map::new();
                map.insert("VisibilityViolation".into(), msg.to_json());
                JsonValue::Object(map)
            }
        }
    }
}

impl std::fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyzerError::CrateNotFound(name) => write!(f, "Crate not found: {}", name),
            AnalyzerError::ParseError { file, message } => {
                write!(f, "Parse error in {:?}: {}", file, message)
            }
            AnalyzerError::IoError(msg) => write!(f, "IO error: {}", msg),
            AnalyzerError::NotImplemented(feature) => {
                write!(f, "Not implemented: {}", feature)
            }
            AnalyzerError::VisibilityViolation(msg) => {
                write!(f, "Visibility violation: {}", msg)
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AnalyzerError>;
