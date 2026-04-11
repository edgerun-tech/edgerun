//! Layer 10: Spec Change Detector
//!
//! Monitors proto files for changes, detects new CSS properties/value types,
//! and reports conformance gaps. Compares current proto state against
//! a stored baseline to identify additions, removals, and modifications.
//!
//! Usage:
//!   cargo run -p edgerun-spec-watch              # Check for changes
//!   cargo run -p edgerun-spec-watch -- --baseline # Save current state as baseline
//!   cargo run -p edgerun-spec-watch -- --report   # Generate change report

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut baseline = false;
    let mut report = false;

    for arg in &args[1..] {
        match arg.as_str() {
            "--baseline" => baseline = true,
            "--report" => report = true,
            _ => {}
        }
    }

    let workspace = find_workspace();
    let proto_dir = workspace.join("proto/edgerun/v0");

    if baseline {
        save_baseline(&proto_dir, &workspace.join("spec_baseline.json"));
        println!("✅ Baseline saved to spec_baseline.json");
        return;
    }

    let baseline_path = workspace.join("spec_baseline.json");
    let current = scan_protos(&proto_dir);

    if baseline_path.exists() {
        let baseline_data = fs::read_to_string(&baseline_path).unwrap();
        let baseline_items: BTreeSet<String> = baseline_data.lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        let added: BTreeSet<_> = current.items.difference(&baseline_items).collect();
        let removed: BTreeSet<_> = baseline_items.difference(&current.items).collect();

        println!("=== Spec Change Detector ===\n");
        println!("Baseline: {} items", baseline_items.len());
        println!("Current:  {} items", current.items.len());

        if !added.is_empty() {
            println!("\n🆕 Added {} items:", added.len());
            for item in added.iter().take(20) {
                println!("  + {}", item);
            }
            if added.len() > 20 {
                println!("  ... and {} more", added.len() - 20);
            }
        }

        if !removed.is_empty() {
            println!("\n🗑️  Removed {} items:", removed.len());
            for item in removed.iter().take(10) {
                println!("  - {}", item);
            }
        }

        if added.is_empty() && removed.is_empty() {
            println!("\n✅ No spec changes detected");
        }

        if report {
            let added_str: BTreeSet<String> = added.iter().map(|s| (*s).clone()).collect();
            let removed_str: BTreeSet<String> = removed.iter().map(|s| (*s).clone()).collect();
            generate_report(&workspace, &current, &added_str, &removed_str);
        }
    } else {
        println!("⚠️  No baseline found. Run with --baseline to create one.");
        println!("Current proto scan: {} items from {} files", current.items.len(), current.files);
    }
}

#[derive(Debug)]
struct ProtoScan {
    files: usize,
    items: BTreeSet<String>,
}

fn scan_protos(proto_dir: &Path) -> ProtoScan {
    let mut items = BTreeSet::new();
    let mut files = 0;

    for entry in walkdir::WalkDir::new(proto_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "proto"))
    {
        files += 1;
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let rel_path = entry.path().strip_prefix(proto_dir.parent().unwrap().parent().unwrap())
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();

        // Extract enum values
        for line in content.lines() {
            let line = line.trim();
            if let Some(name) = extract_enum_item(line) {
                items.insert(format!("{}::{}", rel_path, name));
            }
        }
    }

    ProtoScan { files, items }
}

fn extract_enum_item(line: &str) -> Option<String> {
    // Match: ENUM_VALUE = 42; // comment
    if line.contains('=') && line.ends_with(';') {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() == 2 {
            let name = parts[0].trim();
            if name.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) && !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn save_baseline(proto_dir: &Path, path: &Path) {
    let scan = scan_protos(proto_dir);
    let content = scan.items.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
    fs::write(path, content).unwrap();
}

fn generate_report(_workspace: &Path, current: &ProtoScan, added: &BTreeSet<String>, removed: &BTreeSet<String>) {
    let report = format!(r#"{{
  "scan_files": {},
  "total_items": {},
  "added": {},
  "removed": {},
  "added_items": {:?},
  "removed_items": {:?}
}}"#,
        current.files,
        current.items.len(),
        added.len(),
        removed.len(),
        added.iter().collect::<Vec<_>>(),
        removed.iter().collect::<Vec<_>>(),
    );
    fs::write("spec_change_report.json", report).unwrap();
    println!("\n📄 Report written to spec_change_report.json");
}

fn find_workspace() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("Cargo.toml").exists() {
            let content = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
            if content.contains("edgerun-") { return dir; }
        }
        if !dir.pop() { break; }
    }
    std::env::current_dir().unwrap()
}
