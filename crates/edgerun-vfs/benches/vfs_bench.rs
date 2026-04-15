use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgerun_vfs::{FineGrainedVFS, GitAwarePersist, VirtualFileSystem};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

fn create_test_files(dir: &Path, count: usize) {
    for i in 0..count {
        let content = format!(
            "fn test_{}() {{\n    let x = {};\n    let y = x + 1;\n    y\n}}\n",
            i, i
        );
        std::fs::write(dir.join(format!("file_{}.rs", i)), content).unwrap();
    }
}

fn create_large_files(dir: &Path, count: usize, lines_per_file: usize) {
    for i in 0..count {
        let mut content = String::with_capacity(lines_per_file * 40);
        for j in 0..lines_per_file {
            content.push_str(&format!(
                "// line {} of file {}: Lorem ipsum dolor sit amet\n",
                j, i
            ));
        }
        std::fs::write(dir.join(format!("large_{}.rs", i)), content).unwrap();
    }
}

fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("load");
    group.warm_up_time(Duration::from_secs(1));

    for &size in [100, 500, 1000].iter() {
        let tmp = TempDir::new().unwrap();
        create_test_files(tmp.path(), size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_function(BenchmarkId::new("files", size), |b| {
            b.iter(|| {
                let vfs = VirtualFileSystem::load(black_box(tmp.path())).unwrap();
                black_box(&vfs);
            })
        });
    }

    group.finish();
}

fn bench_load_vs_disk(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_vs_disk_read");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), 500);

    group.bench_function("vfs_load_500_files", |b| {
        b.iter(|| {
            let vfs = VirtualFileSystem::load(black_box(tmp.path())).unwrap();
            black_box(&vfs);
        })
    });

    group.bench_function("disk_read_500_files", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for entry in std::fs::read_dir(tmp.path()).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        total += content.len();
                    }
                }
            }
            black_box(total);
        })
    });

    group.finish();
}

fn bench_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), 1000);
    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();

    group.throughput(Throughput::Elements(vfs.files().count() as u64));
    group.bench_function("read_all_vfs", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for path in vfs.files() {
                if let Some(content) = vfs.read_str(path) {
                    total += content.len();
                }
            }
            black_box(total);
        })
    });

    group.bench_function("read_all_disk", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for path in vfs.files() {
                let full_path = vfs.root().join(path);
                if let Ok(content) = std::fs::read_to_string(&full_path) {
                    total += content.len();
                }
            }
            black_box(total);
        })
    });

    group.finish();
}

fn bench_read_arc_vs_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_semantics");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), 1000);
    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let paths: Vec<_> = vfs.files().collect();

    group.bench_function("read_arc_1000", |b| {
        b.iter(|| {
            for path in &paths {
                if let Some(content) = vfs.read(path) {
                    black_box(content.as_str().len());
                }
            }
        })
    });

    group.bench_function("read_str_1000", |b| {
        b.iter(|| {
            for path in &paths {
                if let Some(content) = vfs.read_str(path) {
                    black_box(content.len());
                }
            }
        })
    });

    group.finish();
}

fn bench_grep(c: &mut Criterion) {
    let mut group = c.benchmark_group("grep");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), 1000);
    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();

    group.throughput(Throughput::Elements(1000));
    group.bench_function("vfs_grep_fn_test", |b| {
        b.iter(|| {
            let matches = vfs.grep("fn test_");
            black_box(matches.len());
        })
    });

    group.bench_function("disk_grep_fn_test", |b| {
        b.iter(|| {
            let output = std::process::Command::new("grep")
                .args([
                    "-r",
                    "--line-number",
                    "fn test_",
                    tmp.path().to_str().unwrap(),
                ])
                .output();
            let count = output
                .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
                .unwrap_or(0);
            black_box(count);
        })
    });

    group.finish();
}

fn bench_grep_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("grep_patterns");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), 1000);
    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();

    for pattern in &["fn ", "let ", "impl ", "pub fn test_\\d+", "use "] {
        group.bench_function(
            format!("grep_{}", pattern.replace(char::is_whitespace, "_")),
            |b| {
                b.iter(|| {
                    let matches = vfs.grep(pattern);
                    black_box(matches.len());
                })
            },
        );
    }

    group.finish();
}

fn bench_edit(c: &mut Criterion) {
    let mut group = c.benchmark_group("edit");
    group.warm_up_time(Duration::from_secs(1));

    group.bench_function("vfs_edit_single_file", |b| {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("edit_test.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();
        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();

        b.iter(|| {
            vfs.edit(Path::new("edit_test.rs"), |content| {
                content.push_str("\n// edited");
                Ok(())
            })
            .unwrap();
            black_box(&vfs);
        })
    });

    group.bench_function("vfs_edit_100_files", |b| {
        let tmp = TempDir::new().unwrap();
        create_test_files(tmp.path(), 100);
        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        let paths: Vec<_> = vfs.files().take(100).cloned().collect();

        b.iter(|| {
            for path in &paths {
                vfs.edit(path, |content| {
                    content.push_str("\n// edited");
                    Ok(())
                })
                .unwrap();
            }
            black_box(&vfs);
        })
    });

    group.bench_function("disk_edit_100_files", |b| {
        let tmp = TempDir::new().unwrap();
        create_test_files(tmp.path(), 100);

        b.iter(|| {
            for i in 0..100 {
                let path = tmp.path().join(format!("file_{}.rs", i));
                if let Ok(mut content) = std::fs::read_to_string(&path) {
                    content.push_str("\n// edited");
                    let _ = std::fs::write(&path, &content);
                }
            }
        })
    });

    group.finish();
}

fn bench_concurrent_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_read");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), 1000);
    let vfs_rwlock = Arc::new(std::sync::RwLock::new(
        VirtualFileSystem::load(tmp.path()).unwrap(),
    ));
    let fine_vfs = Arc::new(FineGrainedVFS::load(tmp.path()).unwrap());

    group.bench_function("rwlock_8_readers_grep", |b| {
        b.iter(|| {
            let mut handles = vec![];
            for _ in 0..8 {
                let vfs = vfs_rwlock.clone();
                handles.push(std::thread::spawn(move || {
                    let vfs = vfs.read().unwrap();
                    vfs.grep("fn test_").len()
                }));
            }
            let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
            black_box(total)
        })
    });

    group.bench_function("dashmap_8_readers_grep", |b| {
        b.iter(|| {
            let mut handles = vec![];
            for _ in 0..8 {
                let vfs = fine_vfs.clone();
                handles.push(std::thread::spawn(move || vfs.grep("fn test_").len()));
            }
            let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
            black_box(total)
        })
    });

    group.finish();
}

fn bench_gitignore(c: &mut Criterion) {
    let mut group = c.benchmark_group("gitignore_filter");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join(".gitignore"),
        "target/\nnode_modules/\n*.log\n.env\nbuild/\ndist/\n",
    )
    .unwrap();

    let persist = GitAwarePersist::new(tmp.path());

    let should_persist = vec![
        std::path::PathBuf::from("src/main.rs"),
        std::path::PathBuf::from("Cargo.toml"),
        std::path::PathBuf::from("lib/core.rs"),
    ];
    let should_not = vec![
        std::path::PathBuf::from("target/debug/main"),
        std::path::PathBuf::from("target/release/app"),
        std::path::PathBuf::from("node_modules/react/index.js"),
        std::path::PathBuf::from("debug.log"),
        std::path::PathBuf::from(".env"),
        std::path::PathBuf::from("build/output.o"),
    ];

    group.bench_function("should_persist_hit", |b| {
        b.iter(|| {
            for path in &should_persist {
                black_box(persist.should_persist(path));
            }
        })
    });

    group.bench_function("should_persist_miss", |b| {
        b.iter(|| {
            for path in &should_not {
                black_box(persist.should_persist(path));
            }
        })
    });

    group.bench_function("should_persist_mixed_1000", |b| {
        b.iter(|| {
            for _ in 0..100 {
                for path in &should_persist {
                    black_box(persist.should_persist(path));
                }
                for path in &should_not {
                    black_box(persist.should_persist(path));
                }
            }
        })
    });

    group.finish();
}

fn bench_persist(c: &mut Criterion) {
    let mut group = c.benchmark_group("persist");
    group.warm_up_time(Duration::from_secs(1));
    group.sample_size(20);

    group.bench_function("persist_100_dirty_files", |b| {
        b.iter(|| {
            let tmp = TempDir::new().unwrap();
            create_test_files(tmp.path(), 100);
            let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();

            let paths: Vec<_> = vfs.files().take(100).cloned().collect();
            for path in &paths {
                vfs.edit(path, |content| {
                    content.push_str("\n// dirty");
                    Ok(())
                })
                .unwrap();
            }

            let result = vfs.persist().unwrap();
            black_box(result.persisted);
        })
    });

    group.finish();
}

fn bench_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_files");
    group.warm_up_time(Duration::from_secs(1));
    group.sample_size(20);

    let tmp = TempDir::new().unwrap();
    create_large_files(tmp.path(), 10, 10000);

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();

    group.bench_function("grep_10_large_files", |b| {
        b.iter(|| {
            let matches = vfs.grep("Lorem ipsum");
            black_box(matches.len());
        })
    });

    group.bench_function("read_all_10_large_files", |b| {
        b.iter(|| {
            let mut total = 0;
            for path in vfs.files() {
                if let Some(content) = vfs.read_str(path) {
                    total += content.len();
                }
            }
            black_box(total);
        })
    });

    group.bench_function("edit_large_file", |b| {
        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        let first_file = vfs.files().next().unwrap().clone();

        b.iter(|| {
            vfs.edit(&first_file, |content| {
                content.push_str("\n// appended");
                Ok(())
            })
            .unwrap();
            black_box(&vfs);
        })
    });

    group.finish();
}

fn bench_find_files_containing(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_files");
    group.warm_up_time(Duration::from_secs(1));

    let tmp = TempDir::new().unwrap();
    create_test_files(tmp.path(), 1000);
    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();

    group.bench_function("vfs_find_test_0", |b| {
        b.iter(|| {
            let matches = vfs.find_files_containing("test_0");
            black_box(matches.len());
        })
    });

    group.bench_function("vfs_find_fn", |b| {
        b.iter(|| {
            let matches = vfs.find_files_containing("fn");
            black_box(matches.len());
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_load,
    bench_load_vs_disk,
    bench_read,
    bench_read_arc_vs_str,
    bench_grep,
    bench_grep_patterns,
    bench_edit,
    bench_concurrent_read,
    bench_gitignore,
    bench_persist,
    bench_large_files,
    bench_find_files_containing,
);
criterion_main!(benches);
