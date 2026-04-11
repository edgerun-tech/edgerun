//! Edgerun GPU vs CPU rasterizer benchmark.
//!
//! Renders a test scene with both CPU and GPU rasterizers,
//! compares pixel-by-pixel, and generates a report.

use std::time::Instant;

use edgerun_bench_correctness::{build_test_scene, rasterize_cpu, rasterize_gpu, WIDTH, HEIGHT};

fn main() {
    println!("=== Edgerun GPU vs CPU Rasterizer Benchmark ===\n");

    let scene = build_test_scene();
    println!("Scene: {} rects, {} text commands\n", scene.rects.len(), scene.texts.len());

    println!("--- CPU Rasterization ---");
    let cpu_start = Instant::now();
    let cpu_pixels = rasterize_cpu(&scene, WIDTH, HEIGHT);
    let cpu_elapsed = cpu_start.elapsed();
    println!("Time: {:.3} ms", cpu_elapsed.as_secs_f64() * 1000.0);
    edgerun_bench_correctness::diff_engine::save_png("edgerun_bench_cpu.png", &cpu_pixels, WIDTH, HEIGHT);

    println!("\n--- GPU Rasterization ---");
    let gpu_start = Instant::now();
    let gpu_pixels = rasterize_gpu(&scene, WIDTH, HEIGHT);
    let gpu_elapsed = gpu_start.elapsed();
    println!("Time: {:.3} ms", gpu_elapsed.as_secs_f64() * 1000.0);
    edgerun_bench_correctness::diff_engine::save_png("edgerun_bench_gpu.png", &gpu_pixels, WIDTH, HEIGHT);

    println!("\n--- Pixel Comparison ---");
    let config = edgerun_bench_correctness::diff_engine::DiffConfig::default();
    let diff = edgerun_bench_correctness::diff_engine::compare_pixels_with_threshold(
        &cpu_pixels, &gpu_pixels, WIDTH, &config,
    );

    let diff_img = edgerun_bench_correctness::diff_engine::generate_diff_image(&cpu_pixels, &gpu_pixels, WIDTH, &config);
    edgerun_bench_correctness::diff_engine::save_png("edgerun_bench_diff.png", &diff_img, WIDTH, HEIGHT);

    println!("Total pixels:    {}", diff.total_pixels);
    println!("Exact matches:   {} ({:.2}%)", diff.exact_matches,
             diff.exact_matches as f64 / diff.total_pixels as f64 * 100.0);
    println!("Near matches:    {} (diff ≤ 4, {:.2}%)", diff.near_matches,
             diff.near_matches as f64 / diff.total_pixels as f64 * 100.0);
    println!("Mismatches:      {} (diff > 4, {:.4}%)", diff.mismatches,
             diff.mismatches as f64 / diff.total_pixels as f64 * 100.0);
    if diff.max_channel_diff > 0 {
        println!("Max channel diff: {}", diff.max_channel_diff);
    }

    let speedup = cpu_elapsed.as_secs_f64() / gpu_elapsed.as_secs_f64();
    println!("\n--- Performance ---");
    println!("CPU:  {:.3} ms", cpu_elapsed.as_secs_f64() * 1000.0);
    println!("GPU:  {:.3} ms", gpu_elapsed.as_secs_f64() * 1000.0);
    if speedup > 1.0 {
        println!("GPU is {:.2}x faster than CPU", speedup);
    } else {
        println!("CPU is {:.2}x faster than GPU", 1.0 / speedup);
    }

    let report = edgerun_bench_correctness::diff_engine::build_report(
        &cpu_pixels, &gpu_pixels,
        cpu_elapsed.as_secs_f64() * 1000.0,
        gpu_elapsed.as_secs_f64() * 1000.0,
        WIDTH, HEIGHT, &config, None,
    );
    edgerun_bench_correctness::diff_engine::save_report(&report, "edgerun_bench_report.json");
    println!("\nReport written to edgerun_bench_report.json");

    if !report.ci_pass {
        if let Some(ref reason) = report.ci_failure_reason {
            println!("\n⚠️  CI check warning: {}", reason);
        }
    }
}
