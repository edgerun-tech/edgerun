//! Hot-reload proto → regenerate → re-render.
//!
//! Watches proto files for changes and automatically:
//! 1. Runs the appropriate generator script
//! 2. Rebuilds affected crates
//! 3. Re-runs the demo
//! 4. Shows a diff between old and new output
//!
//! Usage:
//!   cargo run -p edgerun-proto-watch              # Watch default proto dir
//!   cargo run -p edgerun-proto-watch -- --once     # Single regeneration
//!   cargo run -p edgerun-proto-watch -- --no-demo  # Skip demo render

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut once = false;
    let mut run_demo = true;

    for arg in &args[1..] {
        match arg.as_str() {
            "--once" => once = true,
            "--no-demo" => run_demo = false,
            "--help" | "-h" => {
                println!("Usage: cargo run -p edgerun-proto-watch [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --once      Run regeneration once and exit");
                println!("  --no-demo   Skip demo re-rendering");
                println!("  --help, -h  Show this help");
                return;
            }
            _ => {}
        }
    }

    let workspace_root = find_workspace_root();
    let proto_dir = workspace_root.join("proto");

    if !proto_dir.exists() {
        eprintln!("Proto directory not found: {}", proto_dir.display());
        eprintln!("Run from the workspace root directory.");
        std::process::exit(1);
    }

    if once {
        regenerate_and_render(&workspace_root, run_demo);
        return;
    }

    println!("=== Edgerun Proto Hot-Reload ===");
    println!("Watching: {}", proto_dir.display());
    println!("Press Ctrl+C to stop\n");

    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        tx,
        Config::default().with_poll_interval(std::time::Duration::from_secs(1)),
    )
    .expect("Failed to create file watcher");

    watcher
        .watch(&proto_dir, RecursiveMode::Recursive)
        .expect("Failed to watch proto directory");

    // Initial render
    regenerate_and_render(&workspace_root, run_demo);

    // Watch loop
    for event in rx {
        let event = match event {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Watch error: {:?}", e);
                continue;
            }
        };

        // Filter to modified .proto files
        let proto_files: Vec<_> = event
            .paths
            .iter()
            .filter(|p| p.extension().map_or(false, |e| e == "proto"))
            .filter(|_p| matches!(event.kind, EventKind::Modify(_)))
            .collect();

        if proto_files.is_empty() {
            continue;
        }

        println!("\n┌─ Proto file change detected ──────────────────────");
        for p in &proto_files {
            println!("│  {}", p.strip_prefix(&workspace_root).unwrap_or(p).display());
        }
        println!("└───────────────────────────────────────────────────\n");

        regenerate_and_render(&workspace_root, run_demo);
    }
}

fn find_workspace_root() -> PathBuf {
    // Start from current directory and look for Cargo.toml with "edgerun" members
    let mut dir = std::env::current_dir().expect("Failed to get current directory");
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml).unwrap_or_default();
            if content.contains("edgerun-") {
                return dir;
            }
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir().expect("Failed to get current directory")
}

fn regenerate_and_render(workspace_root: &PathBuf, run_demo: bool) {
    let start = Instant::now();

    // Step 1: Regenerate type crates from proto files
    println!("[1/3] Regenerating type crates from proto files...");
    let buf_result = Command::new("buf")
        .arg("generate")
        .current_dir(workspace_root)
        .output();

    match buf_result {
        Ok(output) => {
            if output.status.success() {
                println!("      ✅ buf generate completed");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("      ⚠️  buf generate had issues: {}", stderr.lines().next().unwrap_or(""));
            }
        }
        Err(e) => {
            println!("      ⚠️  buf not found or failed: {}", e);
            println!("      (Skipping — type crates may already be up to date)");
        }
    }

    // Step 2: Rebuild affected crates
    println!("[2/3] Building crates...");
    let build_result = Command::new("cargo")
        .args(["build", "-p", "edgerun-rasterizer", "-p", "edgerun-demo"])
        .current_dir(workspace_root)
        .output();

    match build_result {
        Ok(output) => {
            if output.status.success() {
                println!("      ✅ build completed");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let errors: Vec<_> = stderr.lines().filter(|l| l.contains("error")).collect();
                println!("      ❌ build failed:");
                for err in errors.iter().take(3) {
                    println!("         {}", err);
                }
                return;
            }
        }
        Err(e) => {
            println!("      ❌ build command failed: {}", e);
            return;
        }
    }

    // Step 3: Re-run demo
    if run_demo {
        println!("[3/3] Re-rendering demo...");
        let demo_result = Command::new("cargo")
            .args(["run", "-p", "edgerun-demo", "--quiet"])
            .current_dir(workspace_root)
            .output();

        match demo_result {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("      ✅ demo rendered");
                    // Show key metrics from demo output
                    for line in stdout.lines() {
                        if line.contains("DOM") || line.contains("CSS") || line.contains("Paint") {
                            println!("         {}", line.trim());
                        }
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("      ❌ demo failed:");
                    for line in stderr.lines().take(5) {
                        println!("         {}", line);
                    }
                }
            }
            Err(e) => {
                println!("      ⚠️  demo command failed: {}", e);
            }
        }
    }

    let elapsed = start.elapsed();
    println!("\n  ⏱️  Total: {:.1}s\n", elapsed.as_secs_f64());
}
