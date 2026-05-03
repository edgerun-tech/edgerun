/// Benchmarks for the parsing pipeline:
/// - Single-file parse performance (small, medium, large)
/// - ParserPool vs standalone parser (allocation savings)
/// - Batch parsing across many files
use std::fs;
use std::path::PathBuf;

use codeanalyzer::parser::{parse_file, ParserPool};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

// ─── Test data ─────────────────────────────────────────────────────────

/// Kernel source directory (if present). Used for large-file benchmarks.
fn kernel_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("kernel-src/linux/kernel/sched")
}

/// Sample source directory.
fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples/src")
}

/// Load a source file, panicking if not found.
fn load_source(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| panic!("missing: {}", path.display()))
}

/// Collect all .c/.h/.rs/.ts files from a directory.
fn collect_sources(dir: &PathBuf) -> Vec<(String, String)> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ["c", "h", "rs", "ts", "tsx"].contains(&ext) {
                    if let Ok(source) = fs::read_to_string(&path) {
                        results.push((path.display().to_string(), source));
                    }
                }
            }
        }
    }
    results
}

// ─── Benchmark groups ──────────────────────────────────────────────────

fn bench_single_file_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_file_parse");

    // Small files (samples)
    let small_files = [
        ("samples/api.rs", samples_dir().join("api.rs")),
        ("samples/auth.rs", samples_dir().join("auth.rs")),
        ("samples/app.ts", samples_dir().join("app.ts")),
        ("samples/database.ts", samples_dir().join("database.ts")),
    ];
    for (name, path) in small_files.iter() {
        if path.exists() {
            let source = load_source(path);
            group.bench_with_input(BenchmarkId::new("small", name), &source, |b, src| {
                b.iter(|| parse_file(name, src));
            });
        }
    }

    // Medium kernel files
    let kernel = kernel_dir();
    let med_files = [
        ("kernel/clock.c", kernel.join("clock.c")),
        ("kernel/completion.c", kernel.join("completion.c")),
    ];
    for (name, path) in med_files.iter() {
        if path.exists() {
            let source = load_source(path);
            group.bench_with_input(BenchmarkId::new("medium", name), &source, |b, src| {
                b.iter(|| parse_file(name, src));
            });
        }
    }

    // Large kernel files
    let large_files =
        [("kernel/core.c", kernel.join("core.c")), ("kernel/fair.c", kernel.join("fair.c"))];
    for (name, path) in large_files.iter() {
        if path.exists() {
            let source = load_source(path);
            let loc = source.lines().count();
            group.bench_with_input(
                BenchmarkId::new("large", format!("{name} ({loc} LOC)")),
                &source,
                |b, src| b.iter(|| parse_file(name, src)),
            );
        }
    }

    group.finish();
}

fn bench_parser_pool_vs_standalone(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_reuse");

    // Collect all sample + kernel files
    let mut all_files = collect_sources(&samples_dir());
    all_files.extend(collect_sources(&kernel_dir()));

    if all_files.is_empty() {
        group.bench_function("pool_overhead", |b| b.iter(|| {}));
        group.bench_function("standalone_overhead", |b| b.iter(|| {}));
        group.finish();
        return;
    }

    let file_count = all_files.len();

    group.bench_function(
        BenchmarkId::new("ParserPool_batch", format!("{file_count} files")),
        |b| {
            b.iter_batched(
                || all_files.clone(),
                |files| {
                    let mut pool = ParserPool::new();
                    for (path, source) in &files {
                        let _ = pool.parse_file(path, source);
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );

    group.bench_function(
        BenchmarkId::new("standalone_batch", format!("{file_count} files")),
        |b| {
            b.iter_batched(
                || all_files.clone(),
                |files| {
                    for (path, source) in &files {
                        let _ = parse_file(path, source);
                    }
                },
                BatchSize::SmallInput,
            );
        },
    );

    group.finish();
}

fn bench_batch_small_vs_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_parsing");

    // Small batch: 4 sample files
    let small = collect_sources(&samples_dir());
    if !small.is_empty() {
        group.bench_with_input(
            BenchmarkId::new("ParserPool_small", format!("{} files", small.len())),
            &small,
            |b, files| {
                b.iter(|| {
                    let mut pool = ParserPool::new();
                    for (path, source) in files {
                        let _ = pool.parse_file(path, source);
                    }
                });
            },
        );
    }

    // Large batch: all kernel files
    let large = collect_sources(&kernel_dir());
    if !large.is_empty() {
        let total_loc: usize = large.iter().map(|(_, s)| s.lines().count()).sum();
        group.bench_with_input(
            BenchmarkId::new(
                "ParserPool_kernel",
                format!("{} files, {} LOC", large.len(), total_loc),
            ),
            &large,
            |b, files| {
                b.iter(|| {
                    let mut pool = ParserPool::new();
                    for (path, source) in files {
                        let _ = pool.parse_file(path, source);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_file_parse,
    bench_parser_pool_vs_standalone,
    bench_batch_small_vs_large,
);
criterion_main!(benches);
