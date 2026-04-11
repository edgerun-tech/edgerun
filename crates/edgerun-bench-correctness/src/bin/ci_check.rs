//! CI-mode visual regression check.
//!
//! Runs the CPU vs GPU comparison and checks against a golden image.
//! Exits with code 1 if the mismatch exceeds the configured threshold.
//!
//! Usage:
//!   cargo run --bin ci_check                    # CPU vs GPU only
//!   cargo run --bin ci_check -- --golden ref.png # Compare against golden
//!   cargo run --bin ci_check -- --generate        # Generate golden image
//!   cargo run --bin ci_check -- --max-mismatch 0.01  # Allow 0.01% mismatch

use std::process::ExitCode;

use edgerun_bench_correctness::diff_engine;
use edgerun_bench_correctness::{build_test_scene, rasterize_cpu, rasterize_gpu, WIDTH, HEIGHT};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    
    let mut golden_path: Option<String> = None;
    let mut generate = false;
    let mut max_mismatch_pct = 0.0;
    let mut threshold: u8 = 1;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--golden" => {
                i += 1;
                golden_path = args.get(i).cloned();
            }
            "--generate" => generate = true,
            "--max-mismatch" => {
                i += 1;
                max_mismatch_pct = args.get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);
            }
            "--threshold" => {
                i += 1;
                threshold = args.get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
            }
            _ => {}
        }
        i += 1;
    }

    println!("=== Edgerun CI Visual Regression Check ===\n");

    let scene = build_test_scene();
    println!("Scene: {} rects, {} text commands", scene.rects.len(), scene.texts.len());

    let cpu_pixels = rasterize_cpu(&scene, WIDTH, HEIGHT);
    let gpu_pixels = rasterize_gpu(&scene, WIDTH, HEIGHT);

    let config = diff_engine::DiffConfig {
        threshold,
        max_mismatch_pct,
        allow_near_match: true,
    };

    // Generate golden if requested
    if generate {
        let path = golden_path.as_deref().unwrap_or("golden_reference.png");
        diff_engine::save_png(path, &cpu_pixels, WIDTH, HEIGHT);
        println!("Golden image saved to: {}", path);
        println!("{} pixels saved", cpu_pixels.len() / 4);
        return ExitCode::SUCCESS;
    }

    // CPU vs GPU comparison
    let cpu_vs_gpu = diff_engine::compare_pixels_with_threshold(
        &cpu_pixels, &gpu_pixels, WIDTH, &config,
    );

    println!("CPU vs GPU:");
    println!("  Exact matches:  {} ({:.2}%)", cpu_vs_gpu.exact_matches,
             cpu_vs_gpu.exact_matches as f64 / cpu_vs_gpu.total_pixels as f64 * 100.0);
    println!("  Near matches:   {} ({:.2}%)", cpu_vs_gpu.near_matches,
             cpu_vs_gpu.near_matches as f64 / cpu_vs_gpu.total_pixels as f64 * 100.0);
    println!("  Mismatches:     {} ({:.4}%)", cpu_vs_gpu.mismatches,
             cpu_vs_gpu.mismatches as f64 / cpu_vs_gpu.total_pixels as f64 * 100.0);
    println!("  Max diff:       {}", cpu_vs_gpu.max_channel_diff);

    // Save diff visualization
    let diff_img = diff_engine::generate_diff_image(&cpu_pixels, &gpu_pixels, WIDTH, &config);
    diff_engine::save_png("edgerun_ci_diff.png", &diff_img, WIDTH, HEIGHT);
    println!("\nDiff visualization saved to: edgerun_ci_diff.png");

    // Golden comparison if provided
    let mut cpu_vs_golden_mismatches = 0;
    if let Some(ref path) = golden_path {
        if let Some((golden, gw, _)) = diff_engine::load_png(path) {
            if gw == WIDTH {
                let cpu_vs_golden = diff_engine::compare_pixels_with_threshold(
                    &cpu_pixels, &golden, WIDTH, &config,
                );
                cpu_vs_golden_mismatches = cpu_vs_golden.mismatches;
                println!("\nCPU vs Golden ({}):", path);
                println!("  Exact matches:  {} ({:.2}%)", cpu_vs_golden.exact_matches,
                         cpu_vs_golden.exact_matches as f64 / cpu_vs_golden.total_pixels as f64 * 100.0);
                println!("  Mismatches:     {} ({:.4}%)", cpu_vs_golden.mismatches,
                         cpu_vs_golden.mismatches as f64 / cpu_vs_golden.total_pixels as f64 * 100.0);
            } else {
                println!("\nGolden image width mismatch: expected {}, got {}", WIDTH, gw);
                return ExitCode::FAILURE;
            }
        } else {
            println!("\nGolden image not found: {}", path);
            return ExitCode::FAILURE;
        }
    }

    // CI pass/fail check
    let total_mismatches = cpu_vs_gpu.mismatches + cpu_vs_golden_mismatches;
    let mismatch_pct = total_mismatches as f64 / cpu_vs_gpu.total_pixels as f64 * 100.0;

    println!("\n--- CI Check ---");
    println!("  Total mismatches: {}", total_mismatches);
    println!("  Mismatch percentage: {:.4}%", mismatch_pct);
    println!("  Max allowed: {:.2}%", max_mismatch_pct);

    if mismatch_pct <= max_mismatch_pct {
        println!("\n✅ CI CHECK PASSED");
        ExitCode::SUCCESS
    } else {
        println!("\n❌ CI_CHECK FAILED: {:.4}% > {:.2}%", mismatch_pct, max_mismatch_pct);
        ExitCode::FAILURE
    }
}
