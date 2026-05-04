#[path = "aot/artifact.rs"]
mod artifact;
#[path = "aot/backend_x86_64.rs"]
mod backend_x86_64;
#[path = "aot/ir.rs"]
mod ir;
#[path = "aot/lower.rs"]
mod lower;

use anyhow::{bail, Context, Result};
use artifact::{AotArtifact, CompiledFunction, DecodedAotArtifact};
use backend_x86_64::X86_64Backend;
use edgerun_clap::Parser;
use lower::{lower_module, parse_module};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const TARGET: &str = "x86_64-linux-sysv";
const COMPILER: &str = "edgerun-aot-baseline-v0";

#[derive(Parser, Debug)]
#[command(name = "edgerun-aot", about = "Compile or verify small deterministic EdgeRun WASM AOT artifacts")]
struct Args {
    #[arg(default_value = "app.wasm")]
    file: String,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long)]
    emit_ir: bool,

    #[arg(long)]
    verify_artifact: Option<String>,

    #[arg(long)]
    inspect_artifact: Option<String>,

    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(path) = args.inspect_artifact.as_deref() {
        return inspect_artifact(path);
    }

    if let Some(path) = args.verify_artifact.as_deref() {
        return verify_artifact(&args.file, path, args.verbose);
    }

    compile_artifact(&args)
}

fn compile_artifact(args: &Args) -> Result<()> {
    let input_path = Path::new(&args.file);
    let wasm = std::fs::read(input_path).context("failed to read WASM input")?;

    wasmparser::Validator::new()
        .validate_all(&wasm)
        .context("invalid WASM module")?;

    let module = parse_module(&wasm)?;
    let functions = lower_module(module)?;

    if functions.is_empty() {
        bail!("no baseline-AOT-compilable functions found");
    }

    let mut compiled = Vec::new();
    for ir in functions {
        let code = X86_64Backend::compile(&ir)
            .with_context(|| format!("failed to compile {}", ir.name()))?;
        if args.verbose {
            eprintln!("compiled {}: {} bytes", ir.name(), code.len());
        }
        compiled.push(CompiledFunction {
            index: ir.index,
            name: ir.name(),
            ir,
            code,
        });
    }

    let wasm_sha256: [u8; 32] = Sha256::digest(&wasm).into();
    let artifact = AotArtifact {
        wasm_sha256,
        target: TARGET,
        compiler: COMPILER,
        functions: compiled,
    };

    if args.emit_ir {
        for f in &artifact.functions {
            println!("{}", f.ir.render());
        }
    }

    let output = args
        .output
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.with_extension("eraot"));
    std::fs::write(&output, artifact.encode()).context("failed to write AOT artifact")?;

    println!("PASS: wrote {}", output.display());
    Ok(())
}

fn verify_artifact(wasm_path: &str, artifact_path: &str, verbose: bool) -> Result<()> {
    let wasm = std::fs::read(wasm_path).context("failed to read WASM input")?;
    let expected_hash: [u8; 32] = Sha256::digest(&wasm).into();

    let artifact_bytes = std::fs::read(artifact_path).context("failed to read AOT artifact")?;
    let decoded = DecodedAotArtifact::decode(&artifact_bytes).context("invalid AOT artifact")?;
    decoded.verify_for_wasm_sha256(&expected_hash)?;

    if decoded.target != TARGET {
        bail!(
            "AOT artifact target mismatch: expected {}, found {}",
            TARGET,
            decoded.target
        );
    }

    if decoded.compiler != COMPILER {
        bail!(
            "AOT artifact compiler mismatch: expected {}, found {}",
            COMPILER,
            decoded.compiler
        );
    }

    if verbose {
        eprintln!("artifact: {}", artifact_path);
        eprintln!("target: {}", decoded.target);
        eprintln!("compiler: {}", decoded.compiler);
        eprintln!("functions: {}", decoded.functions.len());
        for f in &decoded.functions {
            eprintln!("  {} #{}: {} bytes", f.name, f.index, f.code.len());
        }
    }

    println!("PASS: verified {} against {}", artifact_path, wasm_path);
    Ok(())
}

fn inspect_artifact(artifact_path: &str) -> Result<()> {
    let artifact_bytes = std::fs::read(artifact_path).context("failed to read AOT artifact")?;
    let decoded = DecodedAotArtifact::decode(&artifact_bytes).context("invalid AOT artifact")?;

    println!("AOT artifact: {}", artifact_path);
    println!("  version: {}", decoded.version);
    println!("  target: {}", decoded.target);
    println!("  compiler: {}", decoded.compiler);
    println!("  wasm_sha256: {}", hex32(&decoded.wasm_sha256));
    println!("  functions: {}", decoded.functions.len());
    for f in &decoded.functions {
        println!("    #{} {}: {} bytes", f.index, f.name, f.code.len());
    }

    Ok(())
}

fn hex32(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
