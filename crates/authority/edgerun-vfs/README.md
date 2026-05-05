# edgerun-vfs

In-memory virtual filesystem with git-aware write-back for transparent RAM acceleration.

## How It Works

```
Programs (cargo, rustc, etc.)
        │
        ▼
┌──────────────────────┐
│  bind mount (tmpfs)  │  ← ALL files in RAM, reads/writes go here
└──────────┬───────────┘
           │
┌──────────▼───────────┐
│  notify watcher      │  ← detects file changes in RAM
└──────────┬───────────┘
           │
┌──────────▼───────────┐
│  GitAwarePersist     │  ← filters: only non-gitignored files pass
└──────────┬───────────┘
           │
┌──────────▼───────────┐
│  async write-back     │  ← batched, non-blocking sync to disk
└──────────┬───────────┘
           │
┌──────────▼───────────┐
│  original filesystem  │  ← source of truth on disk
└──────────────────────┘
```

**Key insight**: ALL files load into RAM as raw bytes — text and binary alike. Binary files are accessible via `read()` / `write_bytes()`. Text operations (`read_str`, `edit`, `grep`) auto-skip non-UTF-8 content. Only non-gitignored files write back to disk. `target/`, `node_modules/`, etc. live entirely in RAM and never touch disk on write-back.

## Two Modes of Operation

### 1. Mount binary (`edgerun-vfs-mount`) — automatic write-back

```sh
sudo edgerun-vfs-mount /path/to/project /mnt/ram --ram-size=32G
```

This does NOT use the VFS library. It uses tmpfs + bind mount:

1. Mounts a tmpfs (RAM disk) at `/tmp/edgerun-vfs-ram`
2. Rsyncs ALL files from source to RAM (`rsync -a`)
3. Bind-mounts the RAM disk over your project directory
4. Watches for changes via `notify` crate (file create/modify/remove events)
5. For each change, checks `GitAwarePersist::should_persist()` — only non-gitignored files sync to disk
6. Write-back is async, batched, and configurable (`--batch-delay-ms`, `--sync-workers`)

In this mode, persistence is **automatic**. Programs read/write to the bind mount transparently. `target/`, `node_modules/`, etc. stay in RAM only and never write back. Ctrl+C triggers graceful unmount.

### 2. Library (`VirtualFileSystem` / `FineGrainedVFS`) — manual persist only

```rust
use edgerun_vfs::VirtualFileSystem;

let mut vfs = VirtualFileSystem::load("/path/to/project")?;

// Changes are in-memory only. Nothing writes to disk automatically.
vfs.edit(Path::new("src/main.rs"), |c| {
    c.push_str("\n// comment");
    Ok(())
})?;

// You MUST call persist() to write changes to disk.
// Without this, changes are LOST when the VFS is dropped.
vfs.persist()?;
```

In library mode, there is **no auto-persist and no file watcher**. The VFS is a pure in-memory snapshot. All reads/writes/edit/grep operate on RAM only. If you want changes to reach disk, you must call `persist()` explicitly. If the process crashes before `persist()`, all changes are gone.

## Library API

```rust
use edgerun_vfs::VirtualFileSystem;
use std::path::Path;

let mut vfs = VirtualFileSystem::load("/path/to/project")?;

// Read text files (returns None for binary files)
if let Some(content) = vfs.read_str(Path::new("src/main.rs")) {
    println!("File size: {} bytes", content.len());
}

// Read binary files (returns Arc<Vec<u8>>)
if let Some(bytes) = vfs.read(Path::new("assets/image.png")) {
    println!("Binary file: {} bytes", bytes.len());
}

// Check if a file is text or binary
vfs.is_text(Path::new("data.bin"));  // false if non-UTF-8

// Copy-on-write text edit (returns Err for binary files)
vfs.edit(Path::new("src/main.rs"), |c| {
    c.push_str("\n// comment");
    Ok(())
})?;

// Write binary data
vfs.write_bytes(Path::new("output.bin"), vec![0u8, 1, 2, 3])?;

// Write string data
vfs.write(Path::new("src/main.rs"), "fn main() {}".to_string())?;

// Parallel grep (binary files auto-skipped)
let matches = vfs.grep("fn main");

// **You must call persist() to write changes to disk.**
// Without this, all edits/writes are lost when the VFS is dropped.
vfs.persist()?;
```

## Architecture

### Core Types

| Type | File | Purpose |
|------|------|---------|
| `VirtualFileSystem` | `vfs.rs` | BTreeMap-backed VFS with COW. Stores `Vec<u8>` (handles binary). Single-writer, multi-reader via `Arc<RwLock<>>`. |
| `FineGrainedVFS` | `fine_grained.rs` | Thread-safe VFS for concurrent reads and writes. Also stores `Vec<u8>`. |
| `GitAwarePersist` | `git_aware.rs` | Reads .gitignore + .edgekeep to decide what syncs to disk. |

### Binary

| Binary | Purpose |
|--------|---------|
| `mount_ramfs` | Mounts tmpfs + bind mount with async git-aware write-back daemon. |

### What was removed (and why)

These were removed as redundant/unused implementations:
- `mount.rs` — FUSE stub, never implemented
- `mount_overlay.rs` — OverlayFS approach that copied files twice
- `daemon.rs` — Systemd daemon with no auto-persist and no file watching
- `overlay.rs` — FUSE overlay stub
- `write_queue.rs` — Generic write queue unused by the actual mount binary
- `multi_threaded.rs` — Test-only module, tests moved to integration

## Performance

Benchmarked on the full edgerun_core workspace (24,017 files / 1.98 GB):

| Metric | Value |
|--------|-------|
| Total files on disk | 24,017 (9,793 text, 14,224 binary) |
| Total size | 1,982.69 MB |
| Files loaded into VFS | 24,017 (all, including binary) |
| VFS memory usage | 1,982.69 MB (1.0x raw, zero overhead) |
| Load time (cold start) | 2.05s |
| Estimated SHA1 hashing | 105ms of the 2.05s |

### Read

| Operation | VFS | Disk | Speedup |
|-----------|-----|------|---------|
| Read all files | 24.97ms | 852.72ms | 34.1x |
| Single file read | ~0us | 3us | 39.6x |
| Grep "fn " | 69.71ms | 2.336s | 33.5x |
| Grep "impl " | 52.07ms | 2.035s | 39.1x |
| Grep "use " | 62.09ms | 1.683s | 27.1x |
| Grep "pub " | 59.11ms | 2.842s | 48.1x |
| Grep "struct " | 43.25ms | 2.264s | 52.3x |
| Find files ("fn main") | 160.65ms | - | - |
| Grep "fn main" | 44.46ms | - | - |

### Write

| Operation | VFS | Disk | Speedup |
|-----------|-----|------|---------|
| Write 100 new files (string) | 295us | 660us | 2.2x |
| Write 100 new files (1KB binary) | 469us | 740us | 1.6x |
| Overwrite 50 existing files | 107us | 269us | 2.5x |
| Edit 50 text files (COW) | 430us | 674us | 1.6x |
| COW edit single file (1k iters) | 1us | - | - |
| Write+read round-trip | 2us/op | 8us/op | 4.2x |
| Bulk write 10K files | 43.86ms (228K/s) | 62.96ms (159K/s) | 1.4x |
| Persist 100 files to disk | 889us | - | 100K/s |

### Concurrent

| Implementation | 8 readers, grep "fn " | Per reader |
|---------------|----------------------|------------|
| RwLock\<VFS\> | 32.80ms total | 4.10ms |
| FineGrainedVFS | 1.688s total | 210.95ms |
| **RwLock speedup** | **51.4x** | - |

| Operation | Concurrency | Throughput |
|-----------|-------------|-----------|
| FineGrainedVFS write | 8 threads x 100 writes | 362K writes/s |
| FineGrainedVFS mixed read+write | 4 writers + 4 readers | 9 ops/s |

### GitAwarePersist

| Metric | Value |
|--------|-------|
| Paths checked per iteration | 24,017 |
| Filter time (all paths) | 116.61ms |
| Per-path cost | 4us |

### Downsides

- **2s startup cost** to load everything into RAM (library) or rsync to tmpfs (mount binary)
- **~2 GB RAM** for a 2 GB codebase (1.0x, near-zero structural overhead)
- **Library mode: changes lost on crash** — you must call `persist()` explicitly, no auto-save
- **Mount mode: data in RAM is ephemeral** — if the machine crashes before write-back flushes, gitignored files (target/, node_modules/) are gone; non-gitignored files are at risk until the async write-back completes
- **Not suitable for very large individual files** (loaded entirely into memory)
- **FineGrainedVFS::read_str()** returns `String` (clone) not `&str` (zero-copy)
- **Library: VFS is a snapshot** — no file watching, doesn't auto-sync from disk changes

## GitAwarePersist

```rust
use edgerun_vfs::GitAwarePersist;

let persist = GitAwarePersist::new(path);

persist.should_persist("src/main.rs")    // true  - source file
persist.should_persist("target/debug/main") // false - gitignored
persist.should_persist(".env")             // false - gitignored
```

`.edgekeep` overrides gitignore — useful for ensuring `.env` files persist
even when gitignored.

## Benchmarks

```bash
cargo run -p edgerun-vfs --features std --bin vfs-benchmark -- /path
```

## License

MIT
