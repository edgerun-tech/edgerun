use std::path::Path;
use crate::crate_model::TestInfo;

pub fn collect_test_info(crate_dir: &Path) -> TestInfo {
    let mut info = TestInfo {
        total: 0, unit_tests: 0, integration_tests: 0,
        doc_tests: 0, ignored: 0,
        passing: None, failing: None,
        last_run_status: None, last_run_commit: None,
    };

    count_tests_in_src(crate_dir, &mut info);
    count_tests_in_tests_dir(crate_dir, &mut info);

    let cargo_toml = crate_dir.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("#[cfg(test)]") || content.contains("test = ") {
                info.unit_tests = info.unit_tests.max(1);
            }
        }
    }

    info.total = info.unit_tests + info.integration_tests + info.doc_tests;

    let _ = try_run_tests(crate_dir, &mut info);

    info
}

fn count_tests_in_src(crate_dir: &Path, info: &mut TestInfo) {
    let src_dir = crate_dir.join("src");
    if !src_dir.exists() { return; }
    count_tests_recursive(&src_dir, info, true);
}

fn count_tests_in_tests_dir(crate_dir: &Path, info: &mut TestInfo) {
    let tests_dir = crate_dir.join("tests");
    if !tests_dir.exists() { return; }
    count_tests_recursive(&tests_dir, info, false);
    info.integration_tests = info.integration_tests.max(1);
}

fn count_tests_recursive(dir: &Path, info: &mut TestInfo, is_unit: bool) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                count_tests_recursive(&path, info, is_unit);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let count = content.lines()
                        .filter(|l| l.contains("#[test]") || l.contains("fn test_"))
                        .count();
                    if is_unit {
                        info.unit_tests += count;
                    } else {
                        info.integration_tests += count;
                    }
                }
            }
        }
    }
}

fn try_run_tests(crate_dir: &Path, info: &mut TestInfo) -> Result<(), String> {
    let output = std::process::Command::new("cargo")
        .args(["test", "--no-run", "--message-format=json"])
        .current_dir(crate_dir)
        .output();

    match output {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.contains("error") {
                info.last_run_status = Some("failed to compile".into());
                return Err("compile error".into());
            }
            info.last_run_status = Some("compiled".into());
        }
        Err(_) => {
            info.last_run_status = Some("unknown".into());
        }
    }

    Ok(())
}
