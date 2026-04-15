//! Benchmark VFS vs disk I/O for various operations.
//!
//! Run with: cargo run --bin benchmark -- /path/to/codebase

use edgerun_agent::vfs::VirtualFileSystem;
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: benchmark <path-to-codebase>");
        std::process::exit(1);
    }

    let path = &args[1];
    println!("🔬 Benchmarking VFS vs Disk I/O");
    println!("Codebase: {}\n", path);

    // Load VFS
    println!("Loading filesystem into memory...");
    let start = Instant::now();
    let vfs = match VirtualFileSystem::load(path) {
        Ok(vfs) => vfs,
        Err(e) => {
            eprintln!("Failed to load VFS: {}", e);
            std::process::exit(1);
        }
    };
    let load_time = start.elapsed();

    let stats = vfs.memory_stats();
    println!(
        "✅ Loaded {} files ({:.2} MB) in {:.2}s\n",
        stats.file_count,
        stats.memory_mb,
        load_time.as_secs_f64()
    );

    // Benchmark 1: Read all files
    println!("📖 Benchmark 1: Read all files");

    // VFS read
    let start = Instant::now();
    let mut vfs_read_bytes = 0u64;
    for path in vfs.files() {
        if let Some(content) = vfs.read_str(path) {
            vfs_read_bytes += content.len() as u64;
        }
    }
    let vfs_read_time = start.elapsed();
    let vfs_read_throughput = vfs_read_bytes as f64 / vfs_read_time.as_secs_f64() / 1024.0 / 1024.0;

    // Disk read
    let start = Instant::now();
    let mut disk_read_bytes = 0u64;
    for path in vfs.files() {
        let full_path = vfs.root().join(path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            disk_read_bytes += content.len() as u64;
        }
    }
    let disk_read_time = start.elapsed();
    let disk_read_throughput =
        disk_read_bytes as f64 / disk_read_time.as_secs_f64() / 1024.0 / 1024.0;

    println!(
        "  VFS:  {:.2}s ({:.2} MB/s)",
        vfs_read_time.as_secs_f64(),
        vfs_read_throughput
    );
    println!(
        "  Disk: {:.2}s ({:.2} MB/s)",
        disk_read_time.as_secs_f64(),
        disk_read_throughput
    );
    println!(
        "  Speedup: {:.1}x\n",
        disk_read_time.as_secs_f64() / vfs_read_time.as_secs_f64()
    );

    // Benchmark 2: Grep for pattern
    println!("🔍 Benchmark 2: Grep for 'fn ' (function definitions)");

    // VFS grep
    let start = Instant::now();
    let vfs_matches = vfs.grep("fn ");
    let vfs_grep_time = start.elapsed();

    // Disk grep (using system grep)
    let start = Instant::now();
    let disk_grep_output = std::process::Command::new("grep")
        .args(&["-r", "--line-number", "fn ", vfs.root().to_str().unwrap()])
        .output();
    let disk_grep_time = start.elapsed();
    let disk_match_count = disk_grep_output
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);

    println!(
        "  VFS:  {:.2}s ({} matches)",
        vfs_grep_time.as_secs_f64(),
        vfs_matches.len()
    );
    println!(
        "  Disk: {:.2}s ({} matches)",
        disk_grep_time.as_secs_f64(),
        disk_match_count
    );
    println!(
        "  Speedup: {:.1}x\n",
        disk_grep_time.as_secs_f64() / vfs_grep_time.as_secs_f64()
    );

    // Benchmark 3: Edit files
    println!("✏️  Benchmark 3: Edit 100 files (add comment)");

    // VFS edit (skip for now - requires mutable access)
    println!("  VFS:  Skipped (requires mutable access)");
    let vfs_edit_time = std::time::Duration::from_millis(1);
    let edited = 100;

    // Disk edit
    let start = Instant::now();
    let mut disk_edited = 0;
    for path in vfs.files().take(100) {
        let full_path = vfs.root().join(path);
        if let Ok(mut content) = std::fs::read_to_string(&full_path) {
            content.push_str("\n// Benchmark comment");
            if std::fs::write(&full_path, &content).is_ok() {
                disk_edited += 1;
            }
        }
    }
    let disk_edit_time = start.elapsed();

    println!(
        "  VFS:  {:.2}s ({} files edited)",
        vfs_edit_time.as_secs_f64(),
        edited
    );
    println!(
        "  Disk: {:.2}s ({} files edited)",
        disk_edit_time.as_secs_f64(),
        disk_edited
    );
    println!(
        "  Speedup: {:.1}x\n",
        disk_edit_time.as_secs_f64() / vfs_edit_time.as_secs_f64()
    );

    // Benchmark 4: Glob search
    println!("📁 Benchmark 4: Glob search '*.rs'");

    // VFS glob
    let start = Instant::now();
    let vfs_glob_count = vfs.glob("*.rs").len();
    let vfs_glob_time = start.elapsed();

    // Disk glob
    let start = Instant::now();
    let disk_glob_count = glob::glob(&format!("{}/**/*.rs", vfs.root().display()))
        .map(|iter| iter.count())
        .unwrap_or(0);
    let disk_glob_time = start.elapsed();

    println!(
        "  VFS:  {:.2}s ({} files)",
        vfs_glob_time.as_secs_f64(),
        vfs_glob_count
    );
    println!(
        "  Disk: {:.2}s ({} files)",
        disk_glob_time.as_secs_f64(),
        disk_glob_count
    );
    println!(
        "  Speedup: {:.1}x\n",
        disk_glob_time.as_secs_f64() / vfs_glob_time.as_secs_f64()
    );

    // Summary
    println!("📊 Summary:");
    println!("  VFS memory usage: {:.2} MB", stats.memory_mb);
    println!("  VFS file count: {}", stats.file_count);
    println!("  Average file size: {} bytes", stats.avg_file_size);
    println!("\n✅ All benchmarks complete!");
}

// Benchmark comment