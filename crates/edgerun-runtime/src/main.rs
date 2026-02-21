// SPDX-License-Identifier: Apache-2.0
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use edgerun_runtime::{
    decode_bundle_from_canonical_bytes, execute_bundle_payload_bytes, RuntimeError,
    RuntimeErrorCode,
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "edgerun-runtime")]
#[command(about = "Deterministic runtime scaffold", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Run {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Replay {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long)]
        expect_output_hash: Option<String>,
    },
    ReplayCorpus {
        #[arg(long)]
        profile: String,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long, default_value_t = 2)]
        runs: u32,
    },
    CalibrateFuel {
        #[arg(long)]
        profile: String,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long, default_value_t = 3)]
        runs: u32,
        #[arg(long, default_value_t = 0.4)]
        max_per_unit_spread: f64,
    },
    SloSmoke {
        #[arg(long)]
        profile: String,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long, default_value_t = 50)]
        runs: u32,
        #[arg(long, default_value_t = 100.0)]
        max_p95_ms: f64,
        #[arg(long, default_value_t = 30.0)]
        min_ops_per_sec: f64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { bundle, output } => run(bundle, output).await?,
        Commands::Replay {
            bundle,
            artifact,
            expect_output_hash,
        } => replay(bundle, artifact, expect_output_hash).await?,
        Commands::ReplayCorpus {
            profile,
            artifact,
            runs,
        } => replay_corpus(profile, artifact, runs).await?,
        Commands::CalibrateFuel {
            profile,
            artifact,
            runs,
            max_per_unit_spread,
        } => calibrate_fuel(profile, artifact, runs, max_per_unit_spread).await?,
        Commands::SloSmoke {
            profile,
            artifact,
            runs,
            max_p95_ms,
            min_ops_per_sec,
        } => slo_smoke(profile, artifact, runs, max_p95_ms, min_ops_per_sec).await?,
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct ReplayCorpusArtifact {
    profile: String,
    host_os: String,
    host_arch: String,
    runs_per_case: u32,
    cases: Vec<ReplayCorpusCaseResult>,
}

#[derive(Debug, Serialize)]
struct ReplayCorpusCaseResult {
    case: String,
    expected: String,
    actual: String,
    stable: bool,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct ReplayArtifact {
    bundle_hash: String,
    ok: bool,
    abi_version: Option<u8>,
    runtime_id: Option<String>,
    output_hash: Option<String>,
    output_len: Option<usize>,
    input_len: Option<usize>,
    max_memory_bytes: Option<u32>,
    max_instructions: Option<u64>,
    fuel_limit: Option<u64>,
    fuel_remaining: Option<u64>,
    error_code: Option<String>,
    error_message: Option<String>,
    trap_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct FuelCalibrationArtifact {
    profile: String,
    host_os: String,
    host_arch: String,
    runs_per_case: u32,
    max_per_unit_spread: f64,
    cases: Vec<FuelCalibrationCase>,
    workloads: Vec<FuelCalibrationWorkloadSummary>,
}

#[derive(Debug, Serialize)]
struct FuelCalibrationCase {
    workload: String,
    units: u32,
    max_instructions: u64,
    fuel_used_samples: Vec<u64>,
    fuel_used_min: u64,
    fuel_used_max: u64,
    fuel_used_mean: f64,
    stable: bool,
}

#[derive(Debug, Serialize)]
struct FuelCalibrationWorkloadSummary {
    workload: String,
    monotonic_fuel_used: bool,
    per_unit_min: f64,
    per_unit_max: f64,
    per_unit_spread: f64,
}

#[derive(Debug, Serialize)]
struct SloSmokeArtifact {
    profile: String,
    host_os: String,
    host_arch: String,
    runs_per_case: u32,
    max_p95_ms: f64,
    min_ops_per_sec: f64,
    overall_ops_per_sec: f64,
    all_passed: bool,
    cases: Vec<SloSmokeCaseResult>,
}

#[derive(Debug, Serialize)]
struct SloSmokeCaseResult {
    case: String,
    expected: String,
    actual: String,
    stable: bool,
    passed: bool,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
}

async fn replay(
    bundle_path: PathBuf,
    artifact_path: PathBuf,
    expect_output_hash: Option<String>,
) -> Result<()> {
    let bundle_bytes = tokio::fs::read(&bundle_path).await?;
    let bundle_hash = hex::encode(edgerun_crypto::compute_bundle_hash(&bundle_bytes));
    let decoded = decode_bundle_from_canonical_bytes(&bundle_bytes).ok();
    let artifact = match edgerun_runtime::execute_bundle_payload_bytes_strict(&bundle_bytes) {
        Ok(report) => ReplayArtifact {
            bundle_hash,
            ok: true,
            abi_version: Some(report.abi_version),
            runtime_id: Some(hex::encode(report.runtime_id)),
            output_hash: Some(hex::encode(report.output_hash)),
            output_len: Some(report.output.len()),
            input_len: Some(report.input_len),
            max_memory_bytes: Some(report.max_memory_bytes),
            max_instructions: Some(report.max_instructions),
            fuel_limit: Some(report.fuel_limit),
            fuel_remaining: Some(report.fuel_remaining),
            error_code: None,
            error_message: None,
            trap_code: None,
        },
        Err(err) => replay_error_artifact(bundle_hash, decoded, err),
    };

    let body = serde_json::to_vec_pretty(&artifact)?;
    tokio::fs::write(&artifact_path, body).await?;
    println!("artifact={}", artifact_path.display());
    println!("ok={}", artifact.ok);
    if let Some(expected) = expect_output_hash {
        verify_expected_output_hash(&artifact, &expected)?;
        println!("expect_output_hash_match=true");
    }
    Ok(())
}

async fn slo_smoke(
    profile: String,
    artifact_path: PathBuf,
    runs: u32,
    max_p95_ms: f64,
    min_ops_per_sec: f64,
) -> Result<()> {
    if runs == 0 {
        bail!("runs must be > 0");
    }
    if max_p95_ms <= 0.0 {
        bail!("max-p95-ms must be > 0");
    }
    if min_ops_per_sec <= 0.0 {
        bail!("min-ops-per-sec must be > 0");
    }

    let cases = replay_corpus_cases()?;
    let suite_start = Instant::now();
    let mut results = Vec::with_capacity(cases.len());
    let mut total_ops: u64 = 0;
    for case in cases {
        let mut observed = Vec::with_capacity(runs as usize);
        let mut latencies_ms = Vec::with_capacity(runs as usize);
        for _ in 0..runs {
            let started = Instant::now();
            let value = match edgerun_runtime::execute_bundle_payload_bytes_strict(&case.bundle) {
                Ok(report) => format!("OK:{}", hex::encode(report.output_hash)),
                Err(err) => format!("ERR:{:?}", err.code),
            };
            observed.push(value);
            latencies_ms.push(started.elapsed().as_secs_f64() * 1000.0);
            total_ops = total_ops.saturating_add(1);
        }

        let stable = observed.windows(2).all(|w| w[0] == w[1]);
        let actual = observed
            .first()
            .cloned()
            .unwrap_or_else(|| "ERR:NoObservation".to_string());
        let expected = match case.expected {
            ExpectedOutcome::OutputHash(hash) => format!("OK:{hash}"),
            ExpectedOutcome::ErrorCode(code) => format!("ERR:{code:?}"),
        };
        let p50_ms = percentile(&latencies_ms, 0.50)?;
        let p95_ms = percentile(&latencies_ms, 0.95)?;
        let max_ms = latencies_ms.iter().copied().reduce(f64::max).unwrap_or(0.0);
        let passed = stable && expected == actual && p95_ms <= max_p95_ms;
        results.push(SloSmokeCaseResult {
            case: case.name.to_string(),
            expected,
            actual,
            stable,
            passed,
            p50_ms,
            p95_ms,
            max_ms,
        });
    }

    let elapsed_secs = suite_start.elapsed().as_secs_f64();
    let overall_ops_per_sec = if elapsed_secs > 0.0 {
        total_ops as f64 / elapsed_secs
    } else {
        0.0
    };
    let all_cases_passed = results.iter().all(|r| r.passed);
    let throughput_ok = overall_ops_per_sec >= min_ops_per_sec;
    let all_passed = all_cases_passed && throughput_ok;

    let artifact = SloSmokeArtifact {
        profile,
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        runs_per_case: runs,
        max_p95_ms,
        min_ops_per_sec,
        overall_ops_per_sec,
        all_passed,
        cases: results,
    };
    let body = serde_json::to_vec_pretty(&artifact)?;
    tokio::fs::write(&artifact_path, body).await?;
    println!("artifact={}", artifact_path.display());
    println!("overall_ops_per_sec={overall_ops_per_sec:.2}");
    println!("all_passed={all_passed}");
    if !all_cases_passed {
        bail!("slo smoke failed: case correctness or latency threshold not met");
    }
    if !throughput_ok {
        bail!(
            "slo smoke failed: overall_ops_per_sec {overall_ops_per_sec} < min_ops_per_sec {min_ops_per_sec}"
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum CalibrationWorkload {
    PureLoop,
    ReadHostcallLoop,
}

struct CalibrationCaseSpec {
    workload: CalibrationWorkload,
    units: u32,
}

async fn calibrate_fuel(
    profile: String,
    artifact_path: PathBuf,
    runs: u32,
    max_per_unit_spread: f64,
) -> Result<()> {
    if runs == 0 {
        bail!("runs must be > 0");
    }
    if !(0.0..=1.0).contains(&max_per_unit_spread) {
        bail!("max-per-unit-spread must be between 0.0 and 1.0");
    }

    let specs = calibration_specs();
    let mut cases = Vec::with_capacity(specs.len());
    for spec in specs {
        let mut samples = Vec::with_capacity(runs as usize);
        for _ in 0..runs {
            let bundle = build_calibration_bundle(spec.workload, spec.units)?;
            let report = edgerun_runtime::execute_bundle_payload_bytes_strict(&bundle)?;
            samples.push(report.fuel_limit.saturating_sub(report.fuel_remaining));
        }
        let fuel_used_min = *samples.iter().min().unwrap_or(&0);
        let fuel_used_max = *samples.iter().max().unwrap_or(&0);
        let fuel_used_mean = if samples.is_empty() {
            0.0
        } else {
            samples.iter().map(|v| *v as f64).sum::<f64>() / samples.len() as f64
        };
        cases.push(FuelCalibrationCase {
            workload: workload_name(spec.workload).to_string(),
            units: spec.units,
            max_instructions: calibration_max_instructions(spec.units),
            fuel_used_samples: samples.clone(),
            fuel_used_min,
            fuel_used_max,
            fuel_used_mean,
            stable: samples.windows(2).all(|w| w[0] == w[1]),
        });
    }

    let workloads = summarize_calibration_workloads(&cases)?;
    let all_stable = cases.iter().all(|c| c.stable);
    let all_monotonic = workloads.iter().all(|w| w.monotonic_fuel_used);
    let spread_ok = workloads
        .iter()
        .all(|w| w.per_unit_spread <= max_per_unit_spread);

    let artifact = FuelCalibrationArtifact {
        profile,
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        runs_per_case: runs,
        max_per_unit_spread,
        cases,
        workloads,
    };
    let body = serde_json::to_vec_pretty(&artifact)?;
    tokio::fs::write(&artifact_path, body).await?;
    println!("artifact={}", artifact_path.display());
    println!("all_stable={all_stable}");
    println!("all_monotonic={all_monotonic}");
    println!("spread_ok={spread_ok}");

    if !all_stable {
        bail!("fuel calibration failed: non-stable case observed");
    }
    if !all_monotonic {
        bail!("fuel calibration failed: non-monotonic fuel usage");
    }
    if !spread_ok {
        bail!("fuel calibration failed: per-unit spread exceeded threshold {max_per_unit_spread}");
    }
    Ok(())
}

fn summarize_calibration_workloads(
    cases: &[FuelCalibrationCase],
) -> Result<Vec<FuelCalibrationWorkloadSummary>> {
    let mut by_workload: std::collections::BTreeMap<&str, Vec<&FuelCalibrationCase>> =
        std::collections::BTreeMap::new();
    for case in cases {
        by_workload
            .entry(case.workload.as_str())
            .or_default()
            .push(case);
    }

    let mut out = Vec::with_capacity(by_workload.len());
    for (workload, mut points) in by_workload {
        points.sort_by_key(|p| p.units);
        if points.is_empty() {
            continue;
        }
        let monotonic_fuel_used = points
            .windows(2)
            .all(|w| w[1].fuel_used_mean >= w[0].fuel_used_mean);
        let per_unit: Vec<f64> = points
            .iter()
            .map(|p| {
                if p.units == 0 {
                    0.0
                } else {
                    p.fuel_used_mean / p.units as f64
                }
            })
            .collect();
        let per_unit_min = per_unit
            .iter()
            .copied()
            .reduce(f64::min)
            .ok_or_else(|| anyhow::anyhow!("empty per-unit set"))?;
        let per_unit_max = per_unit
            .iter()
            .copied()
            .reduce(f64::max)
            .ok_or_else(|| anyhow::anyhow!("empty per-unit set"))?;
        let per_unit_spread = if per_unit_max > 0.0 {
            (per_unit_max - per_unit_min) / per_unit_max
        } else {
            0.0
        };
        out.push(FuelCalibrationWorkloadSummary {
            workload: workload.to_string(),
            monotonic_fuel_used,
            per_unit_min,
            per_unit_max,
            per_unit_spread,
        });
    }
    Ok(out)
}

fn calibration_specs() -> Vec<CalibrationCaseSpec> {
    let units = [100_u32, 500_u32, 1000_u32];
    let mut out = Vec::with_capacity(units.len() * 2);
    for u in units {
        out.push(CalibrationCaseSpec {
            workload: CalibrationWorkload::PureLoop,
            units: u,
        });
        out.push(CalibrationCaseSpec {
            workload: CalibrationWorkload::ReadHostcallLoop,
            units: u,
        });
    }
    out
}

fn workload_name(workload: CalibrationWorkload) -> &'static str {
    match workload {
        CalibrationWorkload::PureLoop => "pure_loop",
        CalibrationWorkload::ReadHostcallLoop => "read_hostcall_loop",
    }
}

fn calibration_max_instructions(units: u32) -> u64 {
    (units as u64).saturating_mul(20_000).max(100_000)
}

fn build_calibration_bundle(workload: CalibrationWorkload, units: u32) -> Result<Vec<u8>> {
    let wasm = match workload {
        CalibrationWorkload::PureLoop => calibration_pure_loop_wasm(units)?,
        CalibrationWorkload::ReadHostcallLoop => calibration_read_hostcall_loop_wasm(units)?,
    };
    encode_bundle(
        wasm,
        b"calibration-input".to_vec(),
        1024 * 1024,
        calibration_max_instructions(units),
    )
}

fn calibration_pure_loop_wasm(units: u32) -> Result<Vec<u8>> {
    let wat = format!(
        r#"(module
            (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
            (memory (export "memory") 1 1)
            (func (export "_start")
                (local $i i32)
                (local.set $i (i32.const {units}))
                (loop $loop
                    (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                    (br_if $loop (i32.gt_s (local.get $i) (i32.const 0)))
                )
                (drop (call $write_output (i32.const 0) (i32.const 0)))
            )
        )"#
    );
    Ok(wat::parse_str(&wat)?)
}

fn calibration_read_hostcall_loop_wasm(units: u32) -> Result<Vec<u8>> {
    let wat = format!(
        r#"(module
            (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
            (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
            (memory (export "memory") 1 1)
            (func (export "_start")
                (local $i i32)
                (local.set $i (i32.const {units}))
                (loop $loop
                    (drop (call $read_input (i32.const 0) (i32.const 0) (i32.const 1)))
                    (local.set $i (i32.sub (local.get $i) (i32.const 1)))
                    (br_if $loop (i32.gt_s (local.get $i) (i32.const 0)))
                )
                (drop (call $write_output (i32.const 0) (i32.const 0)))
            )
        )"#
    );
    Ok(wat::parse_str(&wat)?)
}

fn replay_error_artifact(
    bundle_hash: String,
    decoded: Option<edgerun_types::BundlePayload>,
    err: RuntimeError,
) -> ReplayArtifact {
    ReplayArtifact {
        bundle_hash,
        ok: false,
        abi_version: decoded.as_ref().map(|b| b.v),
        runtime_id: decoded.as_ref().map(|b| hex::encode(b.runtime_id)),
        output_hash: None,
        output_len: None,
        input_len: decoded.as_ref().map(|b| b.input.len()),
        max_memory_bytes: decoded.as_ref().map(|b| b.limits.max_memory_bytes),
        max_instructions: decoded.as_ref().map(|b| b.limits.max_instructions),
        fuel_limit: err.fuel_limit,
        fuel_remaining: err.fuel_remaining,
        error_code: Some(format!("{:?}", err.code)),
        error_message: Some(err.message),
        trap_code: err.trap_code,
    }
}

fn verify_expected_output_hash(artifact: &ReplayArtifact, expected_hex: &str) -> Result<()> {
    let expected = expected_hex.trim().to_ascii_lowercase();
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("invalid --expect-output-hash: expected 64 hex chars");
    }
    let Some(actual) = artifact.output_hash.as_ref() else {
        bail!(
            "expected output hash {} but replay did not produce output (ok={})",
            expected,
            artifact.ok
        );
    };
    if *actual != expected {
        bail!("output hash mismatch: expected={expected} actual={actual}");
    }
    Ok(())
}

struct CorpusCase {
    name: &'static str,
    bundle: Vec<u8>,
    expected: ExpectedOutcome,
}

enum ExpectedOutcome {
    OutputHash(String),
    ErrorCode(RuntimeErrorCode),
}

async fn replay_corpus(profile: String, artifact_path: PathBuf, runs: u32) -> Result<()> {
    if runs == 0 {
        bail!("runs must be > 0");
    }

    let cases = replay_corpus_cases()?;
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let mut observed = Vec::with_capacity(runs as usize);
        for _ in 0..runs {
            let value = match edgerun_runtime::execute_bundle_payload_bytes_strict(&case.bundle) {
                Ok(report) => format!("OK:{}", hex::encode(report.output_hash)),
                Err(err) => format!("ERR:{:?}", err.code),
            };
            observed.push(value);
        }

        let stable = observed.windows(2).all(|w| w[0] == w[1]);
        let actual = observed
            .first()
            .cloned()
            .unwrap_or_else(|| "ERR:NoObservation".to_string());
        let expected = match case.expected {
            ExpectedOutcome::OutputHash(hash) => format!("OK:{hash}"),
            ExpectedOutcome::ErrorCode(code) => format!("ERR:{code:?}"),
        };
        let passed = stable && expected == actual;
        results.push(ReplayCorpusCaseResult {
            case: case.name.to_string(),
            expected,
            actual,
            stable,
            passed,
        });
    }

    let artifact = ReplayCorpusArtifact {
        profile,
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        runs_per_case: runs,
        cases: results,
    };

    let all_passed = artifact.cases.iter().all(|c| c.passed);
    let body = serde_json::to_vec_pretty(&artifact)?;
    tokio::fs::write(&artifact_path, body).await?;
    println!("artifact={}", artifact_path.display());
    println!("all_passed={all_passed}");
    if !all_passed {
        bail!("replay corpus failed");
    }
    Ok(())
}

fn replay_corpus_cases() -> Result<Vec<CorpusCase>> {
    let mut cases = Vec::new();

    let echo_wasm = wat::parse_str(
        r#"(module
            (import "edgerun" "input_len" (func $input_len (result i32)))
            (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
            (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
            (memory (export "memory") 1 1)
            (func (export "_start")
                (drop (call $read_input (i32.const 0) (i32.const 0) (call $input_len)))
                (drop (call $write_output (i32.const 0) (call $input_len)))
            )
        )"#,
    )?;
    let echo_input = b"replay-corpus-echo".to_vec();
    let echo_bundle = encode_bundle(echo_wasm, echo_input.clone(), 1024 * 1024, 50_000)?;
    cases.push(CorpusCase {
        name: "echo_input",
        bundle: echo_bundle,
        expected: ExpectedOutcome::OutputHash(hex::encode(edgerun_crypto::blake3_256(&echo_input))),
    });

    let boundary_wasm = wat::parse_str(
        r#"(module
            (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
            (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
            (memory (export "memory") 1 1)
            (func (export "_start")
                (drop (call $read_input (i32.const 65535) (i32.const 2) (i32.const 1)))
                (drop (call $write_output (i32.const 65535) (i32.const 1)))
            )
        )"#,
    )?;
    let boundary_input = b"abc".to_vec();
    let boundary_bundle = encode_bundle(boundary_wasm, boundary_input, 1024 * 1024, 50_000)?;
    cases.push(CorpusCase {
        name: "boundary_write_one_byte",
        bundle: boundary_bundle,
        expected: ExpectedOutcome::OutputHash(hex::encode(edgerun_crypto::blake3_256(b"c"))),
    });

    let fuel_wasm = wat::parse_str(
        r#"(module
            (memory (export "memory") 1 1)
            (func (export "_start")
                (loop
                    br 0
                )
            )
        )"#,
    )?;
    let fuel_bundle = encode_bundle(fuel_wasm, b"fuel".to_vec(), 1024 * 1024, 1_000)?;
    cases.push(CorpusCase {
        name: "instruction_limit_exceeded",
        bundle: fuel_bundle,
        expected: ExpectedOutcome::ErrorCode(RuntimeErrorCode::InstructionLimitExceeded),
    });

    Ok(cases)
}

fn encode_bundle(
    wasm: Vec<u8>,
    input: Vec<u8>,
    max_memory_bytes: u32,
    max_instructions: u64,
) -> Result<Vec<u8>> {
    let payload = edgerun_types::BundlePayload {
        v: edgerun_types::BUNDLE_ABI_CURRENT,
        runtime_id: [9_u8; 32],
        wasm,
        input,
        limits: edgerun_types::Limits {
            max_memory_bytes,
            max_instructions,
        },
        meta: None,
    };
    Ok(edgerun_types::encode_bundle_payload_canonical(&payload)?)
}

fn percentile(values: &[f64], q: f64) -> Result<f64> {
    if values.is_empty() {
        bail!("percentile on empty set");
    }
    if !(0.0..=1.0).contains(&q) {
        bail!("percentile q must be in [0, 1]");
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() - 1) as f64 * q).ceil() as usize;
    Ok(sorted[idx])
}

async fn run(bundle_path: PathBuf, output_path: PathBuf) -> Result<()> {
    let bundle_bytes = tokio::fs::read(&bundle_path).await?;
    let report = execute_bundle_payload_bytes(&bundle_bytes)?;
    tokio::fs::write(&output_path, &report.output).await?;

    println!("bundle_hash={}", hex::encode(report.bundle_hash));
    println!("abi_version={}", report.abi_version);
    println!("runtime_id={}", hex::encode(report.runtime_id));
    println!("output_hash={}", hex::encode(report.output_hash));
    println!("input_len={}", report.input_len);
    println!("max_memory_bytes={}", report.max_memory_bytes);
    println!("max_instructions={}", report.max_instructions);
    println!("fuel_limit={}", report.fuel_limit);
    println!("fuel_remaining={}", report.fuel_remaining);

    Ok(())
}
