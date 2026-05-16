use crate::codealyzer::crate_model::*;
use std::path::{Component, Path, PathBuf};

pub fn static_security_analysis(files: &[PathBuf]) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();

    for file in files {
        if let Ok(content) = std::fs::read_to_string(file) {
            let file_findings = analyze_file_security(file, &content);
            findings.extend(file_findings);
        }
    }

    for (i, f) in findings.iter_mut().enumerate() {
        f.id = format!("SEC-{:03}", i + 1);
    }

    findings
}

fn analyze_file_security(file: &PathBuf, content: &str) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let is_test = is_test_context(file);
    let mut next_item_is_test = false;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("#[test") || trimmed.starts_with("#[cfg(test") {
            next_item_is_test = true;
            continue;
        }
        if next_item_is_test {
            if trimmed.is_empty() || trimmed.starts_with("#[") {
                continue;
            }
            if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                next_item_is_test = false;
                continue;
            }
            next_item_is_test = false;
        }

        if is_test || is_comment_only(trimmed) {
            continue;
        }

        if trimmed.contains(".unwrap()") || trimmed.contains(".expect(") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Low,
                title: "Unwrap/expect in non-test code".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "unwrap()/expect() can panic on None/Err values".into(),
                confidence: Confidence::Likely,
                recommendation: "Handle Option/Result explicitly or use ? operator".into(),
            });
        }

        if trimmed.contains("panic!")
            || trimmed.contains("todo!")
            || trimmed.contains("unimplemented!")
        {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Medium,
                title: "Panic/todo/unimplemented macro".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "These macros terminate execution abruptly".into(),
                confidence: Confidence::Exact,
                recommendation: "Replace with proper error handling".into(),
            });
        }

        if trimmed.contains("std::env::var") || trimmed.contains("env::var") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Info,
                title: "Environment variable access".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Accessing environment variables - may contain secrets".into(),
                confidence: Confidence::Exact,
                recommendation: "Validate and sanitize environment variable usage".into(),
            });
        }

        if trimmed.contains("std::fs::")
            || trimmed.contains("File::open")
            || trimmed.contains("File::create")
        {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Info,
                title: "File system access".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "File system operations - validate paths to prevent traversal".into(),
                confidence: Confidence::Exact,
                recommendation: "Validate file paths and check permissions".into(),
            });
        }

        if trimmed.contains("TODO") || trimmed.contains("FIXME") || trimmed.contains("XXX") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Info,
                title: "TODO/FIXME comment".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Comment indicates incomplete or problematic code".into(),
                confidence: Confidence::Exact,
                recommendation: "Address the flagged issue or create a tracking ticket".into(),
            });
        }

        if contains_secret_identifier(trimmed) {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Medium,
                title: "Potential secret reference".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Variable name suggests sensitive data handling".into(),
                confidence: Confidence::Likely,
                recommendation: "Ensure secrets are not hardcoded or logged".into(),
            });
        }

        if trimmed.contains("recursion") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Info,
                title: "Potential recursion".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Function may recurse - check for stack overflow risk".into(),
                confidence: Confidence::Ambiguous,
                recommendation: "Add recursion limits or convert to iteration".into(),
            });
        }

        if trimmed.contains("from_be_bytes")
            || trimmed.contains("from_le_bytes")
            || trimmed.contains("as u8")
            || trimmed.contains("as u16")
            || trimmed.contains("as u32")
            || trimmed.contains("as u64")
        {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Info,
                title: "Integer conversion".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Integer type conversion - verify no overflow/truncation".into(),
                confidence: Confidence::Likely,
                recommendation: "Use checked arithmetic or explicitly handle edge cases".into(),
            });
        }
    }

    findings
}

fn is_test_context(path: &Path) -> bool {
    if path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if matches!(name.to_str(), Some("tests" | "benches" | "examples"))
        )
    }) {
        return true;
    }

    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            stem == "tests"
                || stem.ends_with("_test")
                || stem.ends_with("_tests")
                || stem.contains("conformance")
        })
        .unwrap_or(false)
}

fn is_comment_only(trimmed: &str) -> bool {
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
}

fn contains_secret_identifier(trimmed: &str) -> bool {
    let lowered = trimmed.to_ascii_lowercase();
    [
        "api_key",
        "api_secret",
        "password",
        "secret",
        "bearer",
        "authorization",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}
