use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AnalyzerError {
    CrateNotFound(String),
    ParseError { file: PathBuf, message: String },
    IoError(String),
    NotImplemented(String),
    VisibilityViolation(String),
}

impl AnalyzerError {
    pub fn to_json_string(&self) -> String {
        match self {
            AnalyzerError::CrateNotFound(name) => {
                json_object(&[("CrateNotFound", json_string(name))])
            }
            AnalyzerError::ParseError { file, message } => json_object(&[
                ("file", json_string(&file.to_string_lossy())),
                ("message", json_string(message)),
            ]),
            AnalyzerError::IoError(msg) => json_object(&[("IoError", json_string(msg))]),
            AnalyzerError::NotImplemented(feature) => {
                json_object(&[("NotImplemented", json_string(feature))])
            }
            AnalyzerError::VisibilityViolation(msg) => {
                json_object(&[("VisibilityViolation", json_string(msg))])
            }
        }
    }
}

fn json_object(fields: &[(&str, String)]) -> String {
    let body = fields
        .iter()
        .map(|(key, value)| format!("{}:{}", json_string(key), value))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{body}}}")
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
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
