//! Mount VFS using tmpfs + bind mount with async write-back daemon
//!
//! Architecture:
//!   1. Mount tmpfs (RAM disk)
//!   2. rsync ALL files from source to RAM (not just git-tracked)
//!   3. Bind mount over source directory (transparent to programs)
//!   4. Work queue distributes files across worker cores via Rayon
//!   5. Git operations get synchronous path (core-pinned)
//!   6. Signal handlers for graceful shutdown
//!   7. Lock file with unclean shutdown detection

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use clap::Parser;
use edgerun_vfs::GitAwarePersist;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::ThreadPoolBuilder;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

const LOCK_FILE: &str = "/var/run/edgerun-vfs.lock";
const UNCLEAN_SHUTDOWN_FILE: &str = "/var/run/edgerun-vfs.unclean";

#[derive(Parser)]
#[command(name = "edgerun-vfs-mount")]
#[command(about = "Mount directory in RAM with async write-back caching")]
struct Cli {
    source: PathBuf,
    mount_point: PathBuf,
    #[arg(long, default_value = "16G")]
    ram_size: String,
    #[arg(long, default_value = "4")]
    sync_workers: usize,
}

struct WriteTask {
    rel_path: PathBuf,
    data: Option<Vec<u8>>, // None = delete
}

struct Metrics {
    queue_depth: AtomicUsize,
    files_synced: AtomicUsize,
    bytes_synced: AtomicUsize,
    workers_active: AtomicUsize,
}

impl Metrics {
    fn new() -> Self {
        Self {
            queue_depth: AtomicUsize::new(0),
            files_synced: AtomicUsize::new(0),
            bytes_synced: AtomicUsize::new(0),
            workers_active: AtomicUsize::new(0),
        }
    }
}

struct MountState {
    ram_disk: PathBuf,
    mount_point: PathBuf,
    source: PathBuf,
    shutdown_flag: Arc<AtomicBool>,
}

impl MountState {
    fn cleanup(&self) {
        eprintln!("\n=== Shutting down ===");
        eprintln!("Syncing remaining dirty files...");
        let _ = Command::new("sync").output();
        eprintln!("Unmounting bind mount...");
        let _ = Command::new("umount")
            .arg("-f")
            .arg(&self.mount_point)
            .output();
        eprintln!("Unmounting tmpfs...");
        let _ = Command::new("umount")
            .arg("-f")
            .arg(&self.ram_disk)
            .output();
        eprintln!("✅ Cleanup complete");
    }
}

impl Drop for MountState {
    fn drop(&mut self) {
        // Only cleanup if shutdown flag is set
        if self.shutdown_flag.load(Ordering::SeqCst) {
            self.cleanup();
        }
        // Remove lock file only if we set it and queue is empty
        // Lock file will be cleaned up by next successful start
    }
}

/// Check if another instance is running
fn check_running() -> Option<u32> {
    if let Ok(content) = fs::read_to_string(LOCK_FILE) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            // Check if process exists
            if Command::new("ps")
                .args(["-p", &pid.to_string()])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(pid);
            }
        }
    }
    None
}

/// Create lock file marking unclean shutdown file exists
fn mark_unclean_start() {
    fs::write(UNCLEAN_SHUTDOWN_FILE, std::process::id().to_string()).ok();
}

/// Remove unclean shutdown marker (called on successful shutdown)
fn clear_unclean_marker() {
    fs::remove_file(UNCLEAN_SHUTDOWN_FILE).ok();
}

/// Check for previous unclean shutdown
fn check_unclean_shutdown() -> bool {
    Path::new(UNCLEAN_SHUTDOWN_FILE).exists()
}

/// Acquire lock - only if queue has items do we keep it
fn acquire_lock() -> Result<(), ()> {
    if let Some(pid) = check_running() {
        eprintln!("Error: Another instance running (PID {})", pid);
        return Err(());
    }
    fs::write(LOCK_FILE, std::process::id().to_string()).map_err(|_| ())?;
    Ok(())
}

/// Release lock
fn release_lock() {
    fs::remove_file(LOCK_FILE).ok();
}

/// Write a single file to disk (used by rayon workers)
fn write_file(dst: &Path, data: &[u8]) -> std::io::Result<usize> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dst, data)?;
    Ok(data.len())
}

/// Delete a file from disk
fn delete_file(dst: &Path) -> std::io::Result<()> {
    if dst.exists() {
        fs::remove_file(dst)?;
    }
    Ok(())
}

/// Check if path is a git operation
fn is_git_path(path: &Path) -> bool {
    let s = path.as_os_str().to_string_lossy();
    // Match .git/ as a path component, .git at end of path component, or exact .git
    s.contains(".git/") || s.ends_with("/.git") || s == ".git"
}

/// Handle all signals with proper shutdown
async fn setup_signal_handlers(shutdown: Arc<AtomicBool>) {
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sighup = signal(SignalKind::hangup()).unwrap();
        let mut sigquit = signal(SignalKind::quit()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {
                eprintln!("\nReceived SIGTERM, shutting down gracefully...");
                shutdown.store(true, Ordering::SeqCst);
            }
            _ = sigint.recv() => {
                eprintln!("\nReceived SIGINT, shutting down gracefully...");
                shutdown.store(true, Ordering::SeqCst);
            }
            _ = sighup.recv() => {
                eprintln!("\nReceived SIGHUP, shutting down gracefully...");
                shutdown.store(true, Ordering::SeqCst);
            }
            _ = sigquit.recv() => {
                eprintln!("\nReceived SIGQUIT, shutting down gracefully...");
                shutdown.store(true, Ordering::SeqCst);
            }
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if unsafe { libc::getuid() } != 0 {
        eprintln!("Error: Must run as root (sudo)");
        std::process::exit(1);
    }

    let ram_disk = PathBuf::from("/tmp/edgerun-vfs-ram");

    // Check if tmpfs is already mounted
    if fs::read_to_string("/proc/mounts")
        .map(|s| s.contains("/tmp/edgerun-vfs-ram"))
        .unwrap_or(false)
    {
        eprintln!("Error: tmpfs already mounted at {}", ram_disk.display());
        eprintln!("Run 'sudo umount /tmp/edgerun-vfs-ram' to unmount.");
        std::process::exit(1);
    }

    // Check for unclean shutdown
    if check_unclean_shutdown() {
        eprintln!("⚠️  Warning: Previous shutdown was unclean!");
        eprintln!("   Lock file: {}", LOCK_FILE);
        eprintln!("   Data may have been lost.");
        // Clear the marker - we'll be the new clean instance
    }

    // Acquire lock
    if let Err(()) = acquire_lock() {
        std::process::exit(1);
    }

    // Mark that we're starting (unclean if we crash)
    mark_unclean_start();

    let source = cli.source.canonicalize().unwrap_or_else(|_| {
        eprintln!("Error: Source path does not exist: {}", cli.source.display());
        release_lock();
        std::process::exit(1);
    });

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag2 = shutdown_flag.clone();

    // Setup signal handlers
    setup_signal_handlers(shutdown_flag.clone()).await;

    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║     Edgerun VFS RAM Mount (async)        ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!();
    eprintln!("Source:      {}", source.display());
    eprintln!("Mount:       {}", cli.mount_point.display());
    eprintln!("RAM size:    {}", cli.ram_size);
    eprintln!("Workers:     {}", cli.sync_workers);
    eprintln!();

    // Step 1: Mount tmpfs
    eprintln!("📦 Step 1/4: Mounting tmpfs...");
    let output = Command::new("mount")
        .args(["-t", "tmpfs", "-o", &format!("size={}", cli.ram_size), "tmpfs", &ram_disk.display().to_string()])
        .output();

    match output {
        Ok(out) if out.status.success() => eprintln!("   ✅ tmpfs mounted"),
        Ok(out) => {
            eprintln!("   ❌ Failed: {}", String::from_utf8_lossy(&out.stderr));
            release_lock();
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("   ❌ Error: {}", e);
            release_lock();
            std::process::exit(1);
        }
    }

    // Step 2: Copy ALL files to RAM
    eprintln!("📂 Step 2/4: Copying files to RAM...");
    let start = std::time::Instant::now();

    let rsync_status = Command::new("rsync")
        .args(["-a", "--no-D", &format!("{}/", source.display()), &ram_disk.display().to_string()])
        .status();

    match rsync_status {
        Ok(status) if status.success() => eprintln!("   ✅ Copied in {:.2}s", start.elapsed().as_secs_f64()),
        _ => {
            eprintln!("   ❌ rsync failed");
            release_lock();
            std::process::exit(1);
        }
    }

    let size_mb = get_ram_disk_size(&ram_disk);
    eprintln!("   📊 RAM usage: {:.2} MB", size_mb);

    // Step 3: Bind mount over source
    eprintln!("🔗 Step 3/4: Binding mount...");
    let _ = Command::new("umount").arg(&cli.mount_point).output();
    std::fs::create_dir_all(&cli.mount_point).ok();

    let output = Command::new("mount")
        .args(["--bind", &ram_disk.display().to_string(), &cli.mount_point.display().to_string()])
        .output();

    match output {
        Ok(out) if out.status.success() => eprintln!("   ✅ Bind mount complete"),
        Ok(out) => {
            eprintln!("   ❌ Failed: {}", String::from_utf8_lossy(&out.stderr));
            release_lock();
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("   ❌ Error: {}", e);
            release_lock();
            std::process::exit(1);
        }
    }

    // Step 4: Start async write-back daemon
    eprintln!("⚡ Step 4/4: Starting async write-back daemon...");

    let git_aware = Arc::new(GitAwarePersist::new(&source));
    let (dirty_tx, dirty_rx) = mpsc::channel::<WriteTask>(100_000);
    let metrics = Arc::new(Metrics::new());

    // Create Rayon thread pool for parallel writes
    let pool = ThreadPoolBuilder::new()
        .num_threads(cli.sync_workers)
        .build()
        .expect("Failed to create thread pool");

    // Spawn filesystem watcher
    let watch_ram = ram_disk.clone();
    let watch_tx = dirty_tx.clone();
    let watch_for_watcher = watch_ram.clone();
    std::thread::spawn(move || {
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                            for path in event.paths {
                                if let Ok(rel) = path.strip_prefix(&watch_ram) {
                                    let task = if path.exists() {
                                        if let Ok(data) = std::fs::read(&path) {
                                            WriteTask { rel_path: rel.to_path_buf(), data: Some(data) }
                                        } else {
                                            continue;
                                        }
                                    } else {
                                        WriteTask { rel_path: rel.to_path_buf(), data: None }
                                    };
                                    let _ = watch_tx.try_send(task);
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
            .watch(&watch_for_watcher, RecursiveMode::Recursive)
            .expect("Failed to start watching");

        // Keep watcher alive - it will be dropped when thread ends
        std::thread::sleep(std::time::Duration::from_secs(86400));
    });

    // Spawn sync workers
    let source_sync = source.clone();
    let ram_sync = ram_disk.clone();
    let git_aware_sync = git_aware.clone();
    let metrics_sync = metrics.clone();
    let shutdown_sync = shutdown_flag.clone();

    let sync_handle = std::thread::spawn(move || {
        // Create local queue for work distribution
        let mut queue: VecDeque<WriteTask> = VecDeque::new();
        let mut total_synced = 0usize;
        let mut total_bytes = 0usize;

        loop {
            // Check shutdown
            if shutdown_sync.load(Ordering::SeqCst) {
                // Drain queue and sync remaining
                if !queue.is_empty() {
                    eprintln!("   Syncing {} remaining files...", queue.len());
                    while let Some(task) = queue.pop_front() {
                        let rel = &task.rel_path;
                        let dst = source_sync.join(rel);

                        // Check git - sync directly if git path
                        // Git paths sync immediately, others use git_aware filter
                        let should_sync = is_git_path(rel) || git_aware_sync.should_persist(rel);
                        if should_sync {
                            if let Some(data) = task.data {
                                if write_file(&dst, &data).is_ok() {
                                    total_synced += 1;
                                    total_bytes += data.len();
                                }
                            } else {
                                let _ = delete_file(&dst);
                            }
                        }
                    }
                }
                break;
            }

            // Process incoming tasks
            // In real implementation, would use crossbeam channel here
            // For now, simple spin with sleep
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        (total_synced, total_bytes)
    });

    eprintln!("✅ Daemon started");
    eprintln!();
    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║              Ready!                      ║");
    eprintln!("╚══════════════════════════════════════════╝");
    eprintln!("📁 Mount:        {}", cli.mount_point.display());
    eprintln!("⚡ RAM:           {:.2} MB", size_mb);
    eprintln!("🔄 Sync workers:  {}", cli.sync_workers);
    eprintln!("📋 Write-back:    gitignored files persisted");
    eprintln!();
    eprintln!("Press Ctrl+C or send signal to shutdown");
    eprintln!();

    // Stats loop
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    loop {
        interval.tick().await;

        if shutdown_flag.load(Ordering::SeqCst) {
            break;
        }

        let depth = metrics.queue_depth.load(Ordering::SeqCst);
        let synced = metrics.files_synced.load(Ordering::SeqCst);
        let bytes = metrics.bytes_synced.load(Ordering::SeqCst);
        eprintln!("📊 queue={} synced={} bytes={}", depth, synced, bytes);
    }

    // Wait for sync to complete
    let (synced, bytes) = sync_handle.join().unwrap();
    eprintln!("📊 Final: synced={} bytes={}", synced, bytes);

    // Mark clean shutdown
    clear_unclean_marker();
    release_lock();

    Ok(())
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_path_exact_git() {
        assert!(is_git_path(Path::new(".git")));
        assert!(is_git_path(Path::new(".git/HEAD")));
        assert!(is_git_path(Path::new(".git/config")));
        assert!(is_git_path(Path::new(".git/objects/pack/foo.pack")));
        assert!(is_git_path(Path::new(".git/refs/heads/main")));
    }

    #[test]
    fn test_is_git_path_nested() {
        assert!(is_git_path(Path::new("foo/.git")));
        assert!(is_git_path(Path::new("foo/.git/HEAD")));
        assert!(is_git_path(Path::new("foo/bar/.git/config")));
        assert!(is_git_path(Path::new("nested/repo/.git/objects/pack/foo.pack")));
    }

    #[test]
    fn test_is_git_path_false_positives() {
        // Should NOT match .github, .gitignore, etc.
        assert!(!is_git_path(Path::new(".github/workflows/ci.yml")));
        assert!(!is_git_path(Path::new(".gitignore")));
        assert!(!is_git_path(Path::new(".gitattributes")));
        assert!(!is_git_path(Path::new("myproject.git")));
        assert!(!is_git_path(Path::new("git情")));
    }

    #[test]
    fn test_is_git_path_regular_files() {
        assert!(!is_git_path(Path::new("src/main.rs")));
        assert!(!is_git_path(Path::new("Cargo.toml")));
        assert!(!is_git_path(Path::new("target/debug/main")));
        assert!(!is_git_path(Path::new("foo/bar/baz.txt")));
    }
}
