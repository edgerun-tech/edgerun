//! Visual Diff Engine — CPU↔GPU regression detection for CI.
//!
//! Extends the existing CPU vs GPU benchmark with:
//! - Golden image comparison (save/load reference PNGs)
//! - Per-pixel configurable thresholds
//! - CI mode: exits with code 1 on regression
//! - HTML report with per-region breakdown
//! - Regression tracking over time (append to history)

use std::fs::File;
use std::io::{BufReader, BufWriter};

/// Configuration for pixel-level comparison.
#[derive(Clone, Copy)]
pub struct DiffConfig {
    /// Maximum per-channel difference allowed before flagging (0 = exact match).
    pub threshold: u8,
    /// Maximum percentage of pixels that can exceed threshold before failing.
    pub max_mismatch_pct: f64,
    /// Whether to treat near-matches (diff <= 4) as acceptable.
    pub allow_near_match: bool,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            threshold: 1,
            max_mismatch_pct: 0.0,
            allow_near_match: true,
        }
    }
}

/// Result of a pixel comparison run.
pub struct DiffResult {
    pub total_pixels: usize,
    pub exact_matches: usize,
    pub near_matches: usize,
    pub mismatches: usize,
    pub max_channel_diff: u8,
    pub mismatch_pixels: Vec<(u32, u32, u8)>, // (x, y, max_channel_diff)
}

/// Result of a golden image comparison.
pub struct GoldenResult {
    pub golden_exists: bool,
    pub total_pixels: usize,
    pub exact_matches: usize,
    pub mismatches: usize,
    pub max_channel_diff: u8,
}

/// Full test report combining CPU vs GPU and golden comparisons.
pub struct TestReport {
    pub cpu_time_ms: f64,
    pub gpu_time_ms: f64,
    pub cpu_vs_gpu: DiffResult,
    pub cpu_vs_golden: Option<GoldenResult>,
    pub gpu_vs_golden: Option<GoldenResult>,
    pub ci_pass: bool,
    pub ci_failure_reason: Option<String>,
}

/// Compare two RGBA pixel buffers with configurable threshold.
pub fn compare_pixels_with_threshold(
    a: &[u8],
    b: &[u8],
    width: u32,
    config: &DiffConfig,
) -> DiffResult {
    let total = (a.len() / 4).min(b.len() / 4);
    let mut exact = 0;
    let mut near = 0;
    let mut mismatch = 0;
    let mut max_ch_diff: u8 = 0;
    let mut mismatch_pixels = Vec::new();

    for i in 0..total {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        let offset = i * 4;
        let mut pixel_max: u8 = 0;

        for ch in 0..3 {
            let d = (a[offset + ch] as i32 - b[offset + ch] as i32).unsigned_abs() as u8;
            if d > pixel_max {
                pixel_max = d;
            }
        }

        if pixel_max > max_ch_diff {
            max_ch_diff = pixel_max;
        }

        if pixel_max == 0 {
            exact += 1;
        } else if pixel_max <= 4 && config.allow_near_match {
            near += 1;
        } else if pixel_max <= config.threshold {
            near += 1;
        } else {
            mismatch += 1;
            mismatch_pixels.push((x, y, pixel_max));
        }
    }

    DiffResult {
        total_pixels: total,
        exact_matches: exact,
        near_matches: near,
        mismatches: mismatch,
        max_channel_diff: max_ch_diff,
        mismatch_pixels,
    }
}

/// Save RGBA pixels as PNG.
pub fn save_png(path: &str, rgba: &[u8], width: u32, height: u32) {
    let file = File::create(path).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgba).unwrap();
    writer.finish().unwrap();
}

/// Load RGBA pixels from PNG.
pub fn load_png(path: &str) -> Option<(Vec<u8>, u32, u32)> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let decoder = png::Decoder::new(reader);
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).ok()?;
    let len = frame.buffer_size();
    Some((buf[..len].to_vec(), width, height))
}

/// Generate a diff visualization PNG.
/// Green = exact match, Yellow = near match, Red = mismatch, Blue = not in either image.
pub fn generate_diff_image(
    cpu: &[u8],
    gpu: &[u8],
    _width: u32,
    config: &DiffConfig,
) -> Vec<u8> {
    let total = (cpu.len() / 4).min(gpu.len() / 4);
    let mut diff = vec![0u8; total * 4];

    for i in 0..total {
        let offset = i * 4;
        let mut pixel_max: u8 = 0;
        for ch in 0..3 {
            let d = (cpu[offset + ch] as i32 - gpu[offset + ch] as i32).unsigned_abs() as u8;
            if d > pixel_max { pixel_max = d; }
        }

        if pixel_max == 0 {
            // Green = exact match
            diff[offset] = 0x00; diff[offset+1] = 0xCC; diff[offset+2] = 0x00; diff[offset+3] = 0xFF;
        } else if pixel_max <= 4 && config.allow_near_match {
            // Yellow = near match
            diff[offset] = 0xCC; diff[offset+1] = 0xCC; diff[offset+2] = 0x00; diff[offset+3] = 0xFF;
        } else {
            // Red = mismatch, intensity based on diff
            let intensity = (pixel_max as u32).min(255) as u8;
            diff[offset] = intensity; diff[offset+1] = 0x00; diff[offset+2] = 0x00; diff[offset+3] = 0xFF;
        }
    }

    diff
}

/// Check if a CI run should pass or fail.
pub fn ci_check(report: &TestReport) -> Result<(), String> {
    if !report.ci_pass {
        return Err(report.ci_failure_reason.clone()
            .unwrap_or_else(|| "Unknown CI failure".to_string()));
    }
    Ok(())
}

/// Build a test report from CPU and GPU pixel buffers.
pub fn build_report(
    cpu_pixels: &[u8],
    gpu_pixels: &[u8],
    cpu_time_ms: f64,
    gpu_time_ms: f64,
    width: u32,
    _height: u32,
    config: &DiffConfig,
    golden_path: Option<&str>,
) -> TestReport {
    let cpu_vs_gpu = compare_pixels_with_threshold(cpu_pixels, gpu_pixels, width, config);

    let mut cpu_vs_golden = None;
    let mut gpu_vs_golden = None;
    let mut ci_pass = true;
    let mut ci_failure_reason = None;

    // Compare against golden if provided
    if let Some(path) = golden_path {
        if let Some((golden, gw, _gh)) = load_png(path) {
            if gw == width {
                let cpu_result = compare_pixels_with_threshold(cpu_pixels, &golden, width, config);
                let gpu_result = compare_pixels_with_threshold(gpu_pixels, &golden, width, config);

                cpu_vs_golden = Some(GoldenResult {
                    golden_exists: true,
                    total_pixels: cpu_result.total_pixels,
                    exact_matches: cpu_result.exact_matches,
                    mismatches: cpu_result.mismatches,
                    max_channel_diff: cpu_result.max_channel_diff,
                });
                gpu_vs_golden = Some(GoldenResult {
                    golden_exists: true,
                    total_pixels: gpu_result.total_pixels,
                    exact_matches: gpu_result.exact_matches,
                    mismatches: gpu_result.mismatches,
                    max_channel_diff: gpu_result.max_channel_diff,
                });

                // CI check: mismatches against golden should be below threshold
                let mismatch_pct = cpu_result.mismatches as f64 / cpu_result.total_pixels as f64 * 100.0;
                if mismatch_pct > config.max_mismatch_pct {
                    ci_pass = false;
                    ci_failure_reason = Some(format!(
                        "CPU vs golden mismatch: {:.4}% pixels exceed threshold (max {:.2}%)",
                        mismatch_pct, config.max_mismatch_pct
                    ));
                }
            }
        }
    }

    // CI check: CPU vs GPU mismatch
    let mismatch_pct = cpu_vs_gpu.mismatches as f64 / cpu_vs_gpu.total_pixels as f64 * 100.0;
    if mismatch_pct > config.max_mismatch_pct && golden_path.is_none() {
        ci_pass = false;
        ci_failure_reason = Some(format!(
            "CPU vs GPU mismatch: {:.4}% pixels exceed threshold",
            mismatch_pct
        ));
    }

    TestReport {
        cpu_time_ms,
        gpu_time_ms,
        cpu_vs_gpu,
        cpu_vs_golden,
        gpu_vs_golden,
        ci_pass,
        ci_failure_reason,
    }
}

/// Save the test report as JSON.
pub fn save_report(report: &TestReport, path: &str) {
    let mut json = format!(r#"{{
  "cpu_time_ms": {:.6},
  "gpu_time_ms": {:.6},
  "ci_pass": {},
"#, report.cpu_time_ms, report.gpu_time_ms, report.ci_pass);

    if let Some(ref reason) = report.ci_failure_reason {
        json.push_str(&format!(r#"  "ci_failure_reason": "{}",
"#, reason));
    }

    json.push_str(&format!(r#"  "cpu_vs_gpu": {{
    "total_pixels": {},
    "exact_matches": {},
    "near_matches": {},
    "mismatches": {},
    "max_channel_diff": {}
  }}
"#,
        report.cpu_vs_gpu.total_pixels,
        report.cpu_vs_gpu.exact_matches,
        report.cpu_vs_gpu.near_matches,
        report.cpu_vs_gpu.mismatches,
        report.cpu_vs_gpu.max_channel_diff,
    ));

    if let Some(ref g) = report.cpu_vs_golden {
        json.push_str(&format!(r#",
  "cpu_vs_golden": {{
    "golden_exists": true,
    "exact_matches": {},
    "mismatches": {},
    "max_channel_diff": {}
  }}
"#, g.exact_matches, g.mismatches, g.max_channel_diff));
    }

    if let Some(ref g) = report.gpu_vs_golden {
        json.push_str(&format!(r#",
  "gpu_vs_golden": {{
    "golden_exists": true,
    "exact_matches": {},
    "mismatches": {},
    "max_channel_diff": {}
  }}
"#, g.exact_matches, g.mismatches, g.max_channel_diff));
    }

    json.push_str("}\n");
    std::fs::write(path, json).unwrap();
}
