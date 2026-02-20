use std::path::PathBuf;

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
        v: 1,
        runtime_id: [9_u8; 32],
        wasm,
        input,
        limits: edgerun_types::Limits {
            max_memory_bytes,
            max_instructions,
        },
    };
    Ok(edgerun_types::encode_bundle_payload_canonical(&payload)?)
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
