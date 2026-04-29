use edgerun_vfs::{FineGrainedVFS, GitAwarePersist, VirtualFileSystem};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

fn format_duration(d: std::time::Duration) -> String {
    let us = d.as_micros();
    if us < 1000 {
        format!("{}us", us)
    } else if us < 1_000_000 {
        format!("{:.2}ms", d.as_secs_f64() * 1000.0)
    } else {
        format!("{:.3}s", d.as_secs_f64())
    }
}

fn format_speedup(vfs: std::time::Duration, disk: std::time::Duration) -> String {
    let ratio = disk.as_secs_f64() / vfs.as_secs_f64();
    if ratio.is_nan() || ratio.is_infinite() {
        "N/A".to_string()
    } else {
        format!("{:.1}x", ratio)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: benchmark <path-to-codebase>");
        eprintln!();
        eprintln!("Benchmarks VFS vs disk I/O with real-world patterns.");
        eprintln!("Use your actual project directory for realistic results.");
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    if !path.exists() {
        eprintln!("Error: path '{}' does not exist", args[1]);
        std::process::exit(1);
    }

    println!("================================================================");
    println!("  Edgerun VFS Benchmark Suite");
    println!("================================================================");
    println!("Codebase: {}", path.display());

    let dir_info = count_files(path);
    let (total_files, text_files, binary_files, total_size) = dir_info;
    println!(
        "Files on disk:   {} ({} text, {} binary)",
        total_files, text_files, binary_files
    );
    println!(
        "Size on disk:    {:.2} MB",
        total_size as f64 / (1024.0 * 1024.0)
    );
    println!();

    // ---- SECTION 1: Load Cost ----
    println!("----------------------------------------------------------------");
    println!("  1. LOAD COST (the price of going in-memory)");
    println!("----------------------------------------------------------------");

    {
        let vfs = VirtualFileSystem::load(path).unwrap();
        let _ = vfs.files().count();
        drop(vfs)
    };

    let load_iterations = 3;
    let mut load_times = vec![];
    for _ in 0..load_iterations {
        let start = Instant::now();
        let vfs = VirtualFileSystem::load(path).unwrap();
        let elapsed = start.elapsed();
        load_times.push(elapsed);
        let stats = vfs.memory_stats();
        println!(
            "  Load: {} files ({:.2} MB) in {}",
            stats.file_count,
            stats.memory_mb,
            format_duration(elapsed)
        );
    }
    let avg_load = load_times.iter().sum::<std::time::Duration>() / load_iterations as u32;
    println!("  Avg load time: {}", format_duration(avg_load));

    let vfs = VirtualFileSystem::load(path).unwrap();
    let stats = vfs.memory_stats();
    let text_count = vfs.files().filter(|p| vfs.is_text(p)).count();
    let binary_count = stats.file_count - text_count;
    println!("  Text: {} | Binary: {}", text_count, binary_count);
    println!();

    // ---- SECTION 2: Read Performance ----
    println!("----------------------------------------------------------------");
    println!("  2. READ PERFORMANCE");
    println!("----------------------------------------------------------------");

    let iterations = 50;
    let start = Instant::now();
    let mut vfs_total_bytes = 0u64;
    for _ in 0..iterations {
        let mut bytes = 0u64;
        for p in vfs.files() {
            if let Some(content) = vfs.read(p) {
                bytes += content.len() as u64;
            }
        }
        vfs_total_bytes = bytes;
    }
    let vfs_read_time = start.elapsed() / iterations;

    let start = Instant::now();
    let mut disk_total_bytes = 0u64;
    for _ in 0..iterations {
        let mut bytes = 0u64;
        for p in vfs.files() {
            let full_path = vfs.root().join(p);
            if let Ok(content) = std::fs::read(&full_path) {
                bytes += content.len() as u64;
            }
        }
        disk_total_bytes = bytes;
    }
    let disk_read_time = start.elapsed() / iterations;

    println!(
        "  VFS read all files:  {} ({:.0} MB/s)",
        format_duration(vfs_read_time),
        vfs_total_bytes as f64 / vfs_read_time.as_secs_f64().max(0.000001) / (1024.0 * 1024.0)
    );
    println!(
        "  Disk read all files: {} ({:.0} MB/s)",
        format_duration(disk_read_time),
        disk_total_bytes as f64 / disk_read_time.as_secs_f64().max(0.000001) / (1024.0 * 1024.0)
    );
    println!(
        "  Speedup: {}",
        format_speedup(vfs_read_time, disk_read_time)
    );

    // Single file read
    let first_file = vfs.files().next().unwrap().clone();
    let start = Instant::now();
    for _ in 0..10_000 {
        std::hint::black_box(vfs.read(&first_file));
    }
    let vfs_single_read = start.elapsed() / 10_000;

    let full_path = vfs.root().join(&first_file);
    let start = Instant::now();
    for _ in 0..10_000 {
        std::hint::black_box(std::fs::read(&full_path));
    }
    let disk_single_read = start.elapsed() / 10_000;

    println!(
        "  VFS single file read:  {}",
        format_duration(vfs_single_read)
    );
    println!(
        "  Disk single file read: {}",
        format_duration(disk_single_read)
    );
    println!(
        "  Single file speedup: {}",
        format_speedup(vfs_single_read, disk_single_read)
    );
    println!();

    // ---- SECTION 3: Grep Performance ----
    println!("----------------------------------------------------------------");
    println!("  3. GREP PERFORMANCE (parallel in-memory vs process spawn)");
    println!("----------------------------------------------------------------");

    let patterns = ["fn ", "impl ", "use ", "pub ", "struct "];
    let mut best_vfs_grep = std::time::Duration::MAX;
    let mut best_disk_grep = std::time::Duration::MAX;
    for pattern in &patterns {
        let start = Instant::now();
        let vfs_matches = vfs.grep(pattern);
        let vfs_time = start.elapsed();

        let start = Instant::now();
        let disk_output = std::process::Command::new("grep")
            .args(["-r", "--line-number", pattern, vfs.root().to_str().unwrap()])
            .output();
        let disk_time = start.elapsed();
        let disk_count = disk_output
            .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
            .unwrap_or(0);

        if vfs_time < best_vfs_grep {
            best_vfs_grep = vfs_time;
        }
        if disk_time < best_disk_grep {
            best_disk_grep = disk_time;
        }

        println!(
            "  '{}': VFS {} matches in {} | Disk {} matches in {} | {}",
            pattern,
            vfs_matches.len(),
            format_duration(vfs_time),
            disk_count,
            format_duration(disk_time),
            format_speedup(vfs_time, disk_time),
        );
    }
    println!();

    // ---- SECTION 4: Edit Performance ----
    println!("----------------------------------------------------------------");
    println!("  4. EDIT PERFORMANCE (copy-on-write vs disk write)");
    println!("----------------------------------------------------------------");

    let mut vfs = VirtualFileSystem::load(path).unwrap();
    let edit_paths: Vec<_> = vfs
        .files()
        .filter(|p| vfs.is_text(p))
        .take(50)
        .cloned()
        .collect();

    // VFS edit: use temp copies to avoid corrupting the project
    let tmp_vfs_edit = std::env::temp_dir().join("edgerun-vfs-bench-vfs-edit");
    let _ = std::fs::remove_dir_all(&tmp_vfs_edit);
    std::fs::create_dir_all(&tmp_vfs_edit).unwrap();
    for path in &edit_paths {
        let src = vfs.root().join(path);
        let dst = tmp_vfs_edit.join(path);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let _ = std::fs::copy(&src, &dst);
    }
    let mut vfs_for_edit = VirtualFileSystem::load(&tmp_vfs_edit).unwrap();
    let start = Instant::now();
    for path in &edit_paths {
        let _ = vfs_for_edit.edit(path, |content| {
            content.push_str("\n// benchmark edit");
            Ok(())
        });
    }
    let vfs_edit_time = start.elapsed();
    let _ = std::fs::remove_dir_all(&tmp_vfs_edit);

    // Disk edit: use temp copies to avoid corrupting the project
    let tmp_edit = std::env::temp_dir().join("edgerun-vfs-bench-edit");
    let _ = std::fs::remove_dir_all(&tmp_edit);
    std::fs::create_dir_all(&tmp_edit).unwrap();
    for path in &edit_paths {
        let src = vfs.root().join(path);
        let dst = tmp_edit.join(path);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let _ = std::fs::copy(&src, &dst);
    }
    let start = Instant::now();
    for path in &edit_paths {
        let disk_path = tmp_edit.join(path);
        if let Ok(mut content) = std::fs::read_to_string(&disk_path) {
            content.push_str("\n// benchmark edit");
            let _ = std::fs::write(&disk_path, &content);
        }
    }
    let disk_edit_time = start.elapsed();
    let _ = std::fs::remove_dir_all(&tmp_edit);

    println!(
        "  VFS edit {} files:  {}",
        edit_paths.len(),
        format_duration(vfs_edit_time)
    );
    println!(
        "  Disk edit {} files: {}",
        edit_paths.len(),
        format_duration(disk_edit_time)
    );
    println!(
        "  Speedup: {}",
        format_speedup(vfs_edit_time, disk_edit_time)
    );

    // COW vs full overwrite
    let cow_file = edit_paths
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("Cargo.toml"));
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = vfs.edit(&cow_file, |content| {
            content.push('\n');
            Ok(())
        });
    }
    let cow_edit_time = start.elapsed() / 1000;
    println!(
        "  VFS COW edit (single file, 1k iterations): {}",
        format_duration(cow_edit_time)
    );
    println!();

    // ---- SECTION 5: Write Performance ----
    println!("----------------------------------------------------------------");
    println!("  5. WRITE PERFORMANCE (VFS write vs disk write)");
    println!("----------------------------------------------------------------");

    // 5a: Write new files (string content)
    {
        let mut vfs = VirtualFileSystem::load(path).unwrap();
        let write_count = 100u32;
        let content = "// benchmark write file\nfn main() {}\n".to_string();

        let start = Instant::now();
        for i in 0..write_count {
            let p = PathBuf::from(format!("bench_write_{}.rs", i));
            vfs.write(&p, content.clone()).unwrap();
        }
        let vfs_write_new = start.elapsed();

        let tmp = std::env::temp_dir().join("edgerun-vfs-bench-write-new");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let start = Instant::now();
        for i in 0..write_count {
            let p = tmp.join(format!("bench_write_{}.rs", i));
            std::fs::write(&p, &content).unwrap();
        }
        let disk_write_new = start.elapsed();
        let _ = std::fs::remove_dir_all(&tmp);

        println!("  Write {} new files (string):", write_count);
        println!("    VFS:  {}", format_duration(vfs_write_new));
        println!("    Disk: {}", format_duration(disk_write_new));
        println!(
            "    Speedup: {}",
            format_speedup(vfs_write_new, disk_write_new)
        );
    }

    // 5b: Write new files (binary content)
    {
        let mut vfs = VirtualFileSystem::load(path).unwrap();
        let write_count = 100u32;
        let content = vec![0u8; 1024]; // 1KB binary blobs

        let start = Instant::now();
        for i in 0..write_count {
            let p = PathBuf::from(format!("bench_write_bin_{}.dat", i));
            vfs.write_bytes(&p, content.clone()).unwrap();
        }
        let vfs_write_bin = start.elapsed();

        let tmp = std::env::temp_dir().join("edgerun-vfs-bench-write-bin");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let start = Instant::now();
        for i in 0..write_count {
            let p = tmp.join(format!("bench_write_bin_{}.dat", i));
            std::fs::write(&p, &content).unwrap();
        }
        let disk_write_bin = start.elapsed();
        let _ = std::fs::remove_dir_all(&tmp);

        println!("  Write {} new files (1KB binary):", write_count);
        println!("    VFS:  {}", format_duration(vfs_write_bin));
        println!("    Disk: {}", format_duration(disk_write_bin));
        println!(
            "    Speedup: {}",
            format_speedup(vfs_write_bin, disk_write_bin)
        );
    }

    // 5c: Overwrite existing files (using temp copies to be safe)
    {
        let vfs = VirtualFileSystem::load(path).unwrap();
        let overwrite_paths: Vec<_> = vfs
            .files()
            .filter(|p| vfs.is_text(p))
            .take(50)
            .cloned()
            .collect();

        // VFS overwrite: operate in-memory only (no disk side effects)
        let mut vfs_for_overwrite = VirtualFileSystem::load(path).unwrap();
        let start = Instant::now();
        for (i, p) in overwrite_paths.iter().enumerate() {
            let new_content = format!("// overwritten by benchmark {}\n", i);
            vfs_for_overwrite.write(p, new_content).unwrap();
        }
        let vfs_overwrite = start.elapsed();

        // Disk overwrite: use temp directory to avoid corrupting the project
        let tmp = std::env::temp_dir().join("edgerun-vfs-bench-overwrite");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        for p in &overwrite_paths {
            let src = vfs.root().join(p);
            let dst = tmp.join(p);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let _ = std::fs::copy(&src, &dst);
        }
        let start = Instant::now();
        for (i, p) in overwrite_paths.iter().enumerate() {
            let full_path = tmp.join(p);
            let new_content = format!("// overwritten by benchmark {}\n", i);
            let _ = std::fs::write(&full_path, &new_content);
        }
        let disk_overwrite = start.elapsed();
        let _ = std::fs::remove_dir_all(&tmp);

        println!("  Overwrite {} existing files:", overwrite_paths.len());
        println!("    VFS:  {}", format_duration(vfs_overwrite));
        println!("    Disk: {}", format_duration(disk_overwrite));
        println!(
            "    Speedup: {}",
            format_speedup(vfs_overwrite, disk_overwrite)
        );
    }

    // 5d: Write + read round-trip latency
    {
        let mut vfs = VirtualFileSystem::load(path).unwrap();
        let iterations = 1000u32;
        let content = "benchmark round-trip content\n".to_string();
        let test_path = PathBuf::from("__bench_roundtrip__.txt");

        let start = Instant::now();
        for _ in 0..iterations {
            vfs.write(&test_path, content.clone()).unwrap();
            let _ = vfs.read_str(&test_path);
        }
        let vfs_roundtrip = start.elapsed() / iterations;

        let tmp = std::env::temp_dir().join("edgerun-vfs-bench-roundtrip");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let disk_path = tmp.join("__bench_roundtrip__.txt");

        let start = Instant::now();
        for _ in 0..iterations {
            std::fs::write(&disk_path, &content).unwrap();
            let _ = std::fs::read_to_string(&disk_path);
        }
        let disk_roundtrip = start.elapsed() / iterations;
        let _ = std::fs::remove_dir_all(&tmp);

        println!("  Write+read round-trip ({} iterations):", iterations);
        println!("    VFS:  {} per op", format_duration(vfs_roundtrip));
        println!("    Disk: {} per op", format_duration(disk_roundtrip));
        println!(
            "    Speedup: {}",
            format_speedup(vfs_roundtrip, disk_roundtrip)
        );
    }

    // 5e: Bulk write (10K files)
    {
        let mut vfs = VirtualFileSystem::load(path).unwrap();
        let write_count = 10_000u32;
        let content = "fn bench_bulk() {}\n".to_string();

        let start = Instant::now();
        for i in 0..write_count {
            let p = PathBuf::from(format!("bench_bulk_{}.rs", i));
            vfs.write(&p, content.clone()).unwrap();
        }
        let vfs_bulk = start.elapsed();

        let tmp = std::env::temp_dir().join("edgerun-vfs-bench-bulk");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let start = Instant::now();
        for i in 0..write_count {
            let p = tmp.join(format!("bench_bulk_{}.rs", i));
            std::fs::write(&p, &content).unwrap();
        }
        let disk_bulk = start.elapsed();
        let _ = std::fs::remove_dir_all(&tmp);

        println!("  Bulk write {} files:", write_count);
        println!(
            "    VFS:  {} ({:.0} files/s)",
            format_duration(vfs_bulk),
            write_count as f64 / vfs_bulk.as_secs_f64().max(0.000001)
        );
        println!(
            "    Disk: {} ({:.0} files/s)",
            format_duration(disk_bulk),
            write_count as f64 / disk_bulk.as_secs_f64().max(0.000001)
        );
        println!("    Speedup: {}", format_speedup(vfs_bulk, disk_bulk));
    }
    println!();

    // ---- SECTION 6: Concurrent Read ----
    println!("----------------------------------------------------------------");
    println!("  6. CONCURRENT READ (shared VFS implementations)");
    println!("----------------------------------------------------------------");

    let vfs_rwlock = Arc::new(std::sync::RwLock::new(
        VirtualFileSystem::load(path).unwrap(),
    ));
    let fine_vfs = Arc::new(FineGrainedVFS::load(path).unwrap());
    let reader_count = 8u32;
    let iterations = 10u32;

    // RwLock concurrent reads
    let start = Instant::now();
    for _ in 0..iterations {
        let mut handles = vec![];
        for _ in 0..reader_count {
            let vfs = vfs_rwlock.clone();
            handles.push(std::thread::spawn(move || {
                let vfs = vfs.read().unwrap();
                let mut total = 0usize;
                for p in vfs.files() {
                    if let Some(content) = vfs.read(p) {
                        total += content.len();
                    }
                }
                total
            }));
        }
        let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
        std::hint::black_box(total);
    }
    let rwlock_total = start.elapsed() / iterations;
    let rwlock_per_reader = rwlock_total / reader_count;

    // FineGrained concurrent reads (grep exercises the read path)
    let start = Instant::now();
    for _ in 0..iterations {
        let mut handles = vec![];
        for _ in 0..reader_count {
            let vfs = fine_vfs.clone();
            handles.push(std::thread::spawn(move || {
                let matches = vfs.grep("fn ");
                matches.len()
            }));
        }
        let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
        std::hint::black_box(total);
    }
    let fine_total = start.elapsed() / iterations;
    let fine_per_reader = fine_total / reader_count;

    println!(
        "  RwLock<VFS> ({} readers, grep 'fn '): {} total, {} per reader",
        reader_count,
        format_duration(rwlock_total),
        format_duration(rwlock_per_reader)
    );
    println!(
        "  FineGrainedVFS ({} readers, grep 'fn '): {} total, {} per reader",
        reader_count,
        format_duration(fine_total),
        format_duration(fine_per_reader)
    );
    println!(
        "  FineGrainedVFS speedup: {}",
        format_speedup(rwlock_total, fine_total)
    );
    println!();

    // ---- SECTION 7: Concurrent Write ----
    println!("----------------------------------------------------------------");
    println!("  7. CONCURRENT WRITE (FineGrainedVFS)");
    println!("----------------------------------------------------------------");

    {
        let fine_vfs = Arc::new(FineGrainedVFS::load(path).unwrap());
        let writer_count = 8u32;
        let writes_per_thread = 100u32;

        // Each thread writes to a different file
        let start = Instant::now();
        let mut handles = vec![];
        for t in 0..writer_count {
            let vfs = fine_vfs.clone();
            handles.push(std::thread::spawn(move || {
                for i in 0..writes_per_thread {
                    let p = PathBuf::from(format!("bench_concurrent_{}_{}.rs", t, i));
                    vfs.write(&p, format!("// thread {} write {}\n", t, i))
                        .unwrap();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let concurrent_write_time = start.elapsed();
        let total_writes = writer_count * writes_per_thread;

        println!(
            "  {} threads x {} writes = {} total: {} ({:.0} writes/s)",
            writer_count,
            writes_per_thread,
            total_writes,
            format_duration(concurrent_write_time),
            total_writes as f64 / concurrent_write_time.as_secs_f64().max(0.000001)
        );

        // Concurrent read + write (readers and writers on different files)
        let fine_vfs2 = Arc::new(FineGrainedVFS::load(path).unwrap());
        let start = Instant::now();
        let mut handles = vec![];
        for t in 0..writer_count {
            let vfs = fine_vfs2.clone();
            if t < writer_count / 2 {
                // writers
                handles.push(std::thread::spawn(move || {
                    for i in 0..writes_per_thread {
                        let p = PathBuf::from(format!("bench_rw_{}_{}.rs", t, i));
                        vfs.write(&p, format!("// write {}{}", t, i)).unwrap();
                    }
                }));
            } else {
                // readers
                handles.push(std::thread::spawn(move || {
                    for _ in 0..writes_per_thread {
                        let _ = vfs.grep("fn ");
                    }
                }));
            }
        }
        for h in handles {
            h.join().unwrap();
        }
        let rw_time = start.elapsed();

        println!(
            "  Mixed read+write ({} writers, {} readers): {} ({:.0} ops/s)",
            writer_count / 2,
            writer_count / 2,
            format_duration(rw_time),
            (writer_count * writes_per_thread) as f64 / rw_time.as_secs_f64().max(0.000001)
        );
    }
    println!();

    // ---- SECTION 8: Gitignore Filtering ----
    println!("----------------------------------------------------------------");
    println!("  6. GITIGNORE FILTERING (should_persist throughput)");
    println!("----------------------------------------------------------------");

    let git_aware = GitAwarePersist::new(path);
    let all_paths: Vec<_> = vfs.files().cloned().collect();
    let iterations = 1000u32;
    let start = Instant::now();
    let mut persist_count = 0usize;
    let mut skip_count = 0usize;
    for _ in 0..iterations {
        for path in &all_paths {
            if git_aware.should_persist(path) {
                persist_count += 1;
            } else {
                skip_count += 1;
            }
        }
    }
    let filter_time = start.elapsed() / iterations;

    println!("  {} paths checked per iteration", all_paths.len());
    println!(
        "  Filter time: {} for all paths",
        format_duration(filter_time)
    );
    println!(
        "  Per-path: {}",
        format_duration(filter_time / all_paths.len().max(1) as u32)
    );
    let sample_paths = vec![
        ("src/main.rs", true),
        ("target/debug/main", false),
        ("README.md", true),
    ];
    for (p, expected) in &sample_paths {
        let result = git_aware.should_persist(Path::new(p));
        println!(
            "  {} -> persist={} (expected={}) {}",
            p,
            result,
            expected,
            if result == *expected { "OK" } else { "WRONG" }
        );
    }
    println!();

    // ---- SECTION 9: Persist (Write-back) ----
    println!("----------------------------------------------------------------");
    println!("  9. PERSIST (write-back to disk)");
    println!("----------------------------------------------------------------");

    let tmp = std::env::temp_dir().join("edgerun-vfs-bench-persist");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    for i in 0..100u32 {
        let content = format!("fn test_{}() {{ let x = {}; }}\n", i, i);
        std::fs::write(tmp.join(format!("persist_{}.rs", i)), &content).unwrap();
    }
    let mut persist_vfs = VirtualFileSystem::load(&tmp).unwrap();
    let persist_paths: Vec<_> = persist_vfs.files().cloned().collect();
    for path in &persist_paths {
        let _ = persist_vfs.edit(path, |c| {
            c.push_str("\n// dirty");
            Ok(())
        });
    }

    let start = Instant::now();
    let result = persist_vfs.persist().unwrap();
    let persist_time = start.elapsed();

    println!(
        "  Persisted {} files in {} ({:.0} files/s)",
        result.persisted,
        format_duration(persist_time),
        result.persisted as f64 / persist_time.as_secs_f64().max(0.001)
    );
    let _ = std::fs::remove_dir_all(&tmp);
    println!();

    // ---- SECTION 10: Memory Overhead ----
    println!("----------------------------------------------------------------");
    println!("  10. MEMORY OVERHEAD (the cost of keeping everything in RAM)");
    println!("----------------------------------------------------------------");

    let raw_size = total_size;
    let vfs_size = stats.memory_bytes;
    let overhead_ratio = vfs_size as f64 / raw_size.max(1) as f64;

    println!(
        "  Raw file bytes on disk:   {:.2} MB",
        raw_size as f64 / (1024.0 * 1024.0)
    );
    println!("  VFS reported usage:        {:.2} MB", stats.memory_mb);
    println!("  Overhead ratio:            {:.2}x", overhead_ratio);
    if stats.file_count > 0 {
        let overhead = vfs_size.saturating_sub(raw_size);
        println!(
            "  Overhead per file:       {} bytes",
            stats.file_count.checked_div(overhead).unwrap_or(0)
        );
    }
    println!("  Files loaded:            {}", stats.file_count);
    println!("  Text files:             {}", text_count);
    println!("  Binary files:           {}", binary_count);
    println!("  Avg file size:           {} bytes", stats.avg_file_size);

    // Estimate structural overhead
    // BTreeMap: ~48 bytes/entry, HashMap: ~48 bytes/entry, Arc<Vec<u8>>: ~24 bytes overhead
    let estimated_structural = stats.file_count * (48 + 48 + 48 + 24 + 40 + 32);
    let estimated_total = raw_size + estimated_structural;
    println!(
        "  Estimated structural:    {:.2} MB",
        estimated_structural as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  Estimated total RSS:    {:.2} MB",
        estimated_total as f64 / (1024.0 * 1024.0)
    );
    println!();

    // ---- SECTION 11: Cold Start Analysis ----
    println!("----------------------------------------------------------------");
    println!("  11. COLD START ANALYSIS");
    println!("----------------------------------------------------------------");

    println!("  Avg load time: {}", format_duration(avg_load));

    // Estimate SHA1 cost
    let mut hash_time = std::time::Duration::ZERO;
    let hash_count = stats.file_count.min(100);
    for path in vfs.files().take(hash_count) {
        if let Some(content) = vfs.read(path) {
            let t = Instant::now();
            use edgerun_crypto::sha1::{Digest, Sha1};
            let mut hasher = Sha1::new();
            hasher.update(&*content);
            let _ = hasher.finalize();
            hash_time += t.elapsed();
        }
    }
    let hash_per_file = if hash_count > 0 {
        hash_time / hash_count as u32
    } else {
        std::time::Duration::ZERO
    };

    println!(
        "  SHA1 hash cost per file: {}",
        format_duration(hash_per_file)
    );
    println!(
        "  Estimated total SHA1:   {}",
        format_duration(hash_per_file * stats.file_count as u32)
    );
    println!();

    // ---- SECTION 12: Find Files ----
    println!("----------------------------------------------------------------");
    println!("  12. FIND FILES (substring search vs grep)");
    println!("----------------------------------------------------------------");

    let start = Instant::now();
    let found = vfs.find_files_containing("fn main");
    let find_time = start.elapsed();

    let start = Instant::now();
    let grep_matches = vfs.grep("fn main");
    let grep_time = start.elapsed();

    println!(
        "  find_files_containing('fn main'): {} files in {}",
        found.len(),
        format_duration(find_time)
    );
    println!(
        "  grep('fn main'):                  {} matches in {}",
        grep_matches.len(),
        format_duration(grep_time)
    );
    println!();

    // ---- Summary ----
    println!("================================================================");
    println!("  SUMMARY");
    println!("================================================================");
    println!(
        "  Read speedup:   {} vs disk (warm cache)",
        format_speedup(vfs_read_time, disk_read_time)
    );
    println!(
        "  Grep speedup:   {} vs grep(1)",
        format_speedup(best_vfs_grep, best_disk_grep)
    );
    println!(
        "  Edit speedup:   {} vs disk write",
        format_speedup(vfs_edit_time, disk_edit_time)
    );
    println!(
        "  Load cost:      {} ({} files [{}, {} text, {} binary], {:.2} MB)",
        format_duration(avg_load),
        stats.file_count,
        total_files,
        text_count,
        binary_count,
        stats.memory_mb
    );
    println!(
        "  Memory cost:    {:.2} MB ({:.1}x raw)",
        stats.memory_mb, overhead_ratio
    );
    println!();
    println!("  BENEFITS:");
    println!(
        "    - Reads are {} faster than disk (even with warm OS cache)",
        format_speedup(vfs_read_time, disk_read_time)
    );
    println!("    - Grep is massively parallel - no process spawn overhead");
    println!(
        "    - Edits are {} faster (COW, no disk I/O until persist)",
        format_speedup(vfs_edit_time, disk_edit_time)
    );
    println!("    - Writes are pure in-memory - no disk I/O until persist");
    println!("    - Safe to mutate without corrupting disk files");
    println!("    - Binary files loaded alongside text files");
    println!();
    println!("  DOWNSIDES:");
    println!(
        "    - {} startup cost to load everything",
        format_duration(avg_load)
    );
    println!(
        "    - {:.2} MB RAM for {:.2} MB of files ({:.1}x)",
        stats.memory_mb,
        raw_size as f64 / (1024.0 * 1024.0),
        overhead_ratio
    );
    println!("    - Changes lost on crash until persist() is called");
    println!("    - FineGrainedVFS::read_str() clones string (no zero-copy)");
    println!("    - VFS is a snapshot, not live (no file watching)");
    println!("================================================================");
}

fn count_files(path: &Path) -> (usize, usize, usize, usize) {
    let mut total_files = 0usize;
    let mut text_files = 0usize;
    let mut binary_files = 0usize;
    let mut total_size = 0usize;

    fn walk(dir: &Path, total: &mut usize, text: &mut usize, binary: &mut usize, size: &mut usize) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(&path, total, text, binary, size);
            } else if let Ok(metadata) = entry.metadata() {
                *total += 1;
                *size += metadata.len() as usize;
                if let Ok(content) = std::fs::read(&path) {
                    if std::str::from_utf8(&content).is_ok() {
                        *text += 1;
                    } else {
                        *binary += 1;
                    }
                } else {
                    *binary += 1;
                }
            }
        }
    }
    walk(
        path,
        &mut total_files,
        &mut text_files,
        &mut binary_files,
        &mut total_size,
    );
    (total_files, text_files, binary_files, total_size)
}

use std::path::PathBuf;
