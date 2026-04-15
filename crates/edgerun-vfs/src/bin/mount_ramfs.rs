//! Mount VFS using tmpfs + bind mount with async write-back daemon
//!
//! Architecture:
//!   1. Mount tmpfs (RAM disk)
//!   2. rsync ALL files from source to RAM (not just git-tracked)
//!   3. Bind mount over source directory (transparent to programs)
//!   4. Async write-back daemon watches for changes in RAM
//!   5. GitAwarePersist filters: only non-gitignored files sync to disk
//!   6. target/, node_modules/ etc stay in RAM only
//!   7. Signal handler for graceful shutdown + unmount

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use clap::Parser;
use edgerun_vfs::GitAwarePersist;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "edgerun-vfs-mount")]
#[command(about = "Mount directory in RAM with async write-back caching")]
struct Cli {
    source: PathBuf,
    mount_point: PathBuf,
    #[arg(long, default_value = "16G")]
    ram_size: String,
    #[arg(long, default_value = "50")]
    batch_delay_ms: u64,
    #[arg(long, default_value = "4")]
    sync_workers: usize,
    #[arg(long, default_value = "1000")]
    max_batch_size: usize,
}

struct MountState {
    ram_disk: PathBuf,
    mount_point: PathBuf,
    source: PathBuf,
}

impl MountState {
    fn cleanup(&self) {
        eprintln!("\n=== Shutting down ===");
        eprintln!("Syncing remaining dirty files...");
        let _ = Command::new("sync").output();
        eprintln!("Unmounting bind mount...");
        let _ = Command::new("umount")
            .arg(&self.mount_point)
            .output();
        eprintln!("Unmounting tmpfs...");
        let _ = Command::new("umount").arg(&self.ram_disk).output();
        eprintln!("✅ Cleanup complete");
    }
}

impl Drop for MountState {
    fn drop(&mut self) {
        self.cleanup();
        // Remove lock file
        std::fs::remove_file("/var/run/edgerun-vfs.pid").ok();
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if unsafe { libc::getuid() } != 0 {
        eprintln!("Error: Must run as root (sudo)");
        std::process::exit(1);
    }

    // Check if tmpfs is already mounted
    let ram_disk = PathBuf::from("/tmp/edgerun-vfs-ram");
    if std::fs::read_to_string("/proc/mounts").ok()
        .map(|s| s.contains("/tmp/edgerun-vfs-ram"))
        .unwrap_or(false)
    {
        eprintln!("Error: tmpfs is already mounted at {}. Another instance may be running.", ram_disk.display());
        eprintln!("Run 'sudo umount /tmp/edgerun-vfs-ram' to unmount, or 'sudo pkill -f edgerun-vfs' to kill other instances.");
        std::process::exit(1);
    }

    // Check for stale lock file
    let lock_file = std::path::PathBuf::from("/var/run/edgerun-vfs.pid");
    if lock_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&lock_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if process is still running
                if std::process::Command::new("ps")
                    .arg("-p")
                    .arg(pid_str.trim())
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    eprintln!("Error: Another instance is running (PID {}). Use 'sudo pkill -f edgerun-vfs' to stop it.", pid);
                    std::process::exit(1);
                }
            }
        }
        // Stale lock file, remove it
        std::fs::remove_file(&lock_file).ok();
    }

    // Write our PID to lock file
    std::fs::write(&lock_file, std::process::id().to_string()).ok();

    let source = cli.source.canonicalize().unwrap_or_else(|_| {
        eprintln!("Error: Source path does not exist: {}", cli.source.display());
        std::process::exit(1);
    });

    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║     Edgerun VFS RAM Mount (async)        ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!();
    eprintln!("Source:      {}", source.display());
    eprintln!("Mount:        {}", cli.mount_point.display());
    eprintln!("RAM size:     {}", cli.ram_size);
    eprintln!("Batch delay:  {}ms", cli.batch_delay_ms);
    eprintln!("Sync workers:  {}", cli.sync_workers);
    eprintln!();

    // Setup cleanup on Ctrl+C
    let state = Arc::new(MountState {
        ram_disk: ram_disk.clone(),
        mount_point: cli.mount_point.clone(),
        source: source.clone(),
    });

    let shutdown_state = state.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        eprintln!("\nReceived Ctrl+C, shutting down...");
        drop(shutdown_state);
        std::process::exit(0);
    });

    // Step 1: Mount tmpfs
    eprintln!("📦 Step 1/4: Mounting tmpfs...");
    let output = Command::new("mount")
        .args([
            "-t",
            "tmpfs",
            "-o",
            &format!("size={}", cli.ram_size),
            "tmpfs",
            &ram_disk.display().to_string(),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => eprintln!("   ✅ tmpfs mounted"),
        Ok(out) => {
            eprintln!("   ❌ Failed: {}", String::from_utf8_lossy(&out.stderr));
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("   ❌ Error: {}", e);
            std::process::exit(1);
        }
    }

    // Step 2: Copy ALL files to RAM
    eprintln!("📂 Step 2/4: Copying files to RAM...");
    let start = Instant::now();

    let rsync_status = Command::new("rsync")
        .args([
            "-a",
            "--no-D",
            &format!("{}/", source.display()),
            &ram_disk.display().to_string(),
        ])
        .status()
        .unwrap_or_else(|e| {
            eprintln!("   ❌ rsync failed: {}", e);
            std::process::exit(1);
        });

    if !rsync_status.success() {
        eprintln!("   ❌ rsync exited with error");
        std::process::exit(1);
    }

    let copy_time = start.elapsed();
    eprintln!("   ✅ Copied in {:.2}s", copy_time.as_secs_f64());

    let size_mb = get_ram_disk_size(&ram_disk);
    eprintln!("   📊 RAM usage: {:.2} MB", size_mb);

    // Step 3: Bind mount over source
    eprintln!("🔗 Step 3/4: Binding mount...");
    let _ = Command::new("umount")
        .arg(&cli.mount_point.display().to_string())
        .output();

    std::fs::create_dir_all(&cli.mount_point).ok();
    let output = Command::new("mount")
        .args([
            "--bind",
            &ram_disk.display().to_string(),
            &cli.mount_point.display().to_string(),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => eprintln!("   ✅ Bind mount complete"),
        Ok(out) => {
            eprintln!("   ❌ Failed: {}", String::from_utf8_lossy(&out.stderr));
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("   ❌ Error: {}", e);
            std::process::exit(1);
        }
    }

    // Step 4: Start async write-back daemon
    eprintln!("⚡ Step 4/4: Starting async write-back daemon...");

    let git_aware = Arc::new(GitAwarePersist::new(&source));
    let (dirty_tx, dirty_rx) = mpsc::channel::<PathBuf>(100_000);

    // Spawn filesystem watcher (blocking, in its own thread)
    let watch_ram = ram_disk.clone();
    let watch_ram2 = ram_disk.clone();
    let watcher_tx = dirty_tx.clone();
    std::thread::spawn(move || {
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                            for path in event.paths {
                                if let Ok(rel) = path.strip_prefix(&watch_ram) {
                                    let _ = watcher_tx.try_send(rel.to_path_buf());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            Config::default(),
        )
        .expect("Failed to create file watcher");

        watcher
            .watch(&watch_ram2, RecursiveMode::Recursive)
            .expect("Failed to start watching");

        std::thread::sleep(Duration::from_secs(86400));
    });

    // Spawn async sync workers
    let source_sync = source.clone();
    let ram_sync = ram_disk.clone();
    let git_aware_sync = git_aware.clone();
    let sync_workers = cli.sync_workers;
    let batch_delay = Duration::from_millis(cli.batch_delay_ms);
    let max_batch = cli.max_batch_size;

    let sync_handle = tokio::spawn(async move {
        sync_loop(
            dirty_rx,
            source_sync,
            ram_sync,
            git_aware_sync,
            batch_delay,
            max_batch,
            sync_workers,
        )
        .await
    });

    eprintln!("✅ Daemon started");
    eprintln!();
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║              Ready!                      ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!("📁 Mount:        {}", cli.mount_point.display());
    eprintln!("⚡ RAM:           {:.2} MB", size_mb);
    eprintln!("💾 Sync delay:    {}ms", cli.batch_delay_ms);
    eprintln!("🔄 Sync workers:  {}", cli.sync_workers);
    eprintln!("📋 Write-back:    non-gitignored files only");
    eprintln!();
    eprintln!("Press Ctrl+C to unmount and exit");
    eprintln!("(target/, node_modules/, etc. stay in RAM only)");
    eprintln!();

    // Stats loop
    let stats_tx = dirty_tx.clone();
    let mut last_count = 0usize;
    let mut total_synced = 0usize;
    let mut total_skipped = 0usize;
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        eprintln!(
            "📊 Stats: synced={} skipped={} (gitignore filtered)",
            total_synced, total_skipped
        );
        last_count = 0;
    }
}

async fn sync_loop(
    mut dirty_rx: mpsc::Receiver<PathBuf>,
    source: PathBuf,
    ram_disk: PathBuf,
    git_aware: Arc<GitAwarePersist>,
    batch_delay: Duration,
    max_batch: usize,
    _workers: usize,
) {
    let mut pending: HashSet<PathBuf> = HashSet::new();
    let mut total_synced: usize = 0;
    let mut total_skipped: usize = 0;
    let mut last_report = Instant::now();

    loop {
        // Collect dirty paths with timeout
        loop {
            match dirty_rx.try_recv() {
                Ok(path) => {
                    pending.insert(path);
                    if pending.len() >= max_batch {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if pending.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        // Wait for batch delay since last sync
        if last_report.elapsed() < batch_delay && pending.len() < max_batch / 2 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            continue;
        }

        // Drain pending into batch
        let batch: Vec<PathBuf> = pending.drain().collect();
        let batch_size = batch.len();

        let mut synced = 0usize;
        let mut skipped = 0usize;

        for rel in &batch {
            if !git_aware.should_persist(rel) {
                skipped += 1;
                continue;
            }

            let src = ram_disk.join(rel);
            let dst = source.join(rel);

            if let Some(parent) = dst.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }

            if src.is_file() || src.is_symlink() {
                match tokio::fs::read(&src).await {
                    Ok(data) => {
                        match tokio::fs::write(&dst, &data).await {
                            Ok(()) => synced += 1,
                            Err(e) => {
                                eprintln!("   ⚠️  Write error for {}: {}", rel.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("   ⚠️  Read error for {}: {}", rel.display(), e);
                    }
                }
            } else if !src.exists() {
                // File was deleted in RAM
                if dst.exists() && git_aware.should_persist(rel) {
                    match tokio::fs::remove_file(&dst).await {
                        Ok(()) => synced += 1,
                        Err(_) => {}
                    }
                }
            }
        }

        total_synced += synced;
        total_skipped += skipped;

        if batch_size > 0 {
            eprintln!(
                "   📝 Batch: {} files (synced={}, skipped=gitignored {})",
                batch_size, synced, skipped
            );
        }

        last_report = Instant::now();
    }
}

fn get_ram_disk_size(path: &Path) -> f64 {
    let output = Command::new("du")
        .args(["-sb", &path.display().to_string()])
        .output();

    output
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .unwrap_or("0")
                .parse::<u64>()
                .ok()
        })
        .map(|b| b as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0)
}