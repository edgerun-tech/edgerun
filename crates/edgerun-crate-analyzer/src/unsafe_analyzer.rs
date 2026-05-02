use std::path::PathBuf;
use crate::crate_model::*;

pub fn find_unsafe_blocks(files: &[PathBuf]) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut count = 0;

    for file in files {
        if let Ok(content) = std::fs::read_to_string(file) {
            let file_findings = scan_file_for_unsafe(file, &content);
            findings.extend(file_findings);
        }
    }

    for (i, f) in findings.iter_mut().enumerate() {
        f.id = format!("UNSAFE-{:03}", i + 1);
    }

    findings
}

fn scan_file_for_unsafe(file: &PathBuf, content: &str) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut in_unsafe_block = false;
    let mut unsafe_start = 0;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.contains("unsafe fn ") || trimmed.contains("pub unsafe fn ") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Medium,
                title: "Unsafe function".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Function uses unsafe keyword - requires manual review".into(),
                confidence: Confidence::Exact,
                recommendation: "Review unsafe function for memory safety".into(),
            });
        }

        if trimmed.contains("unsafe {") {
            in_unsafe_block = true;
            unsafe_start = line_num + 1;
        }

        if in_unsafe_block {
            if trimmed.contains("}") && !trimmed.contains("unsafe") {
                findings.push(SecurityFinding {
                    id: String::new(),
                    severity: Severity::Info,
                    title: "Unsafe block".into(),
                    file: Some(file.clone()),
                    line: Some(unsafe_start),
                    code_excerpt: Some(format!("unsafe block at line {}", unsafe_start)),
                    explanation: "Code within unsafe block can perform unchecked operations".into(),
                    confidence: Confidence::Exact,
                    recommendation: "Ensure unsafe block maintains safety invariants".into(),
                });
                in_unsafe_block = false;
            }
        }

        if trimmed.contains("std::mem::transmute") || trimmed.contains("transmute::<") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::High,
                title: "Transmute usage".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Transmute can reinterpret memory types unsafely".into(),
                confidence: Confidence::Exact,
                recommendation: "Replace with safe type conversions where possible".into(),
            });
        }

        if trimmed.contains("*const ") || trimmed.contains("*mut ") || trimmed.contains("as *mut") || trimmed.contains("as *const") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Medium,
                title: "Raw pointer usage".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Raw pointers bypass Rust's safety guarantees".into(),
                confidence: Confidence::Exact,
                recommendation: "Use references or smart pointers where possible".into(),
            });
        }

        if trimmed.contains("extern \"") || trimmed.contains("extern {") {
            findings.push(SecurityFinding {
                id: String::new(),
                severity: Severity::Low,
                title: "FFI boundary".into(),
                file: Some(file.clone()),
                line: Some(line_num + 1),
                code_excerpt: Some(line.into()),
                explanation: "Foreign function interface - external code boundary".into(),
                confidence: Confidence::Exact,
                recommendation: "Ensure FFI declarations are correct and safe".into(),
            });
        }
    }

    findings
}
