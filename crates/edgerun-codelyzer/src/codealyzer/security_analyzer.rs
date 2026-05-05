use std::path::PathBuf;
use crate::codealyzer::crate_model::*;

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
    let is_test = file.to_string_lossy().contains("/tests/") || content.contains("#[cfg(test)]");

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if is_test { continue; }

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

        if trimmed.contains("panic!") || trimmed.contains("todo!") || trimmed.contains("unimplemented!") {
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

        if trimmed.contains("std::fs::") || trimmed.contains("File::open") || trimmed.contains("File::create") {
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

        if trimmed.contains("secret") || trimmed.contains("password") || trimmed.contains("token") || trimmed.contains("api_key") {
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

        if trimmed.contains("recursion") || (trimmed.contains("fn ") && content.lines().skip(line_num).take(50).any(|l| l.contains("self::") || l.contains("Self::"))) {
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

        if trimmed.contains("from_be_bytes") || trimmed.contains("from_le_bytes") || trimmed.contains("as u8") || trimmed.contains("as u16") || trimmed.contains("as u32") || trimmed.contains("as u64") {
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
