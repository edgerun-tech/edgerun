use std::path::Path;
use crate::crate_model::StandardCoverage;

pub fn load_standards_matrix(crate_dir: &Path, _workspace_root: &Path) -> Vec<StandardCoverage> {
    let mut standards = Vec::new();

    let standards_dir = crate_dir.join("standards");
    if !standards_dir.exists() {
        return vec![StandardCoverage {
            standard: "unknown".into(),
            status: "not provided".into(),
            code_refs: Vec::new(),
            tests: Vec::new(),
            confidence: crate::crate_model::Confidence::Ambiguous,
            notes: Some("No standards matrix found".into()),
        }];
    }

    if let Ok(entries) = std::fs::read_dir(&standards_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(toml) = edgerun_json::from_toml_str(&content) {
                        if let Some(obj) = toml.as_object() {
                            for (key, value) in obj {
                                let status = value.get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                standards.push(StandardCoverage {
                                    standard: key.clone(),
                                    status,
                                    code_refs: Vec::new(),
                                    tests: Vec::new(),
                                    confidence: crate::crate_model::Confidence::Ambiguous,
                                    notes: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if standards.is_empty() {
        standards.push(StandardCoverage {
            standard: "unknown".into(),
            status: "not provided".into(),
            code_refs: Vec::new(),
            tests: Vec::new(),
            confidence: crate::crate_model::Confidence::Ambiguous,
            notes: Some("No RFC matrix found".into()),
        });
    }

    standards
}
