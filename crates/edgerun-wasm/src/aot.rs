#[path = "aot/artifact.rs"]
mod artifact;
#[path = "aot/backend_x86_64_guarded.rs"]
mod backend_x86_64;
#[path = "aot/ir.rs"]
mod ir;
#[path = "aot/loader.rs"]
mod loader;
#[path = "aot/lower.rs"]
mod lower;
#[path = "aot/module_backend.rs"]
mod module_backend;
mod wasmparser_mock;

use anyhow::{bail, Context, Result};
use artifact::{AotArtifact, CompiledFunction, DecodedAotArtifact};
use loader::{find_function, LoadedFunction};
use lower::{lower_module, parse_module};
use module_backend::X86_64ModuleBackend;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const TARGET: &str = "x86_64-linux-sysv";
const COMPILER: &str = "edgerun-aot-baseline-v0";

#[derive(Debug, Default)]
struct Args {
    file: String,
    output: String,
    emit_ir: bool,
    verify_artifact: String,
    inspect_artifact: String,
    run_artifact: String,
    function: String,
    args: String,
    verbose: bool,
}

fn main() -> Result<()> {
    let args = parse_args()?;

    if !args.inspect_artifact.is_empty() {
        return inspect_artifact(&args.inspect_artifact);
    }

    if !args.verify_artifact.is_empty() {
        return verify_artifact(&args.file, &args.verify_artifact, args.verbose);
    }

    if !args.run_artifact.is_empty() {
        return run_artifact(
            &args.file,
            &args.run_artifact,
            &args.function,
            &args.args,
            args.verbose,
        );
    }

    compile_artifact(&args)
}

fn parse_args() -> Result<Args> {
    let mut out = Args {
        file: "app.wasm".to_string(),
        function: "run".to_string(),
        ..Args::default()
    };

    let mut positional = Vec::new();
    let mut it = std::env::args().skip(1).peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-v" | "--verbose" => out.verbose = true,
            "--emit-ir" => out.emit_ir = true,
            "-o" | "--output" => out.output = next_value(&mut it, &arg)?,
            "--verify-artifact" => out.verify_artifact = next_value(&mut it, &arg)?,
            "--inspect-artifact" => out.inspect_artifact = next_value(&mut it, &arg)?,
            "--run-artifact" => out.run_artifact = next_value(&mut it, &arg)?,
            "--function" => out.function = next_value(&mut it, &arg)?,
            "--args" => out.args = next_value(&mut it, &arg)?,
            _ if arg.starts_with("--output=") => out.output = arg[9..].to_string(),
            _ if arg.starts_with("--verify-artifact=") => {
                out.verify_artifact = arg[18..].to_string()
            }
            _ if arg.starts_with("--inspect-artifact=") => {
                out.inspect_artifact = arg[19..].to_string()
            }
            _ if arg.starts_with("--run-artifact=") => out.run_artifact = arg[15..].to_string(),
            _ if arg.starts_with("--function=") => out.function = arg[11..].to_string(),
            _ if arg.starts_with("--args=") => out.args = arg[7..].to_string(),
            _ if arg.starts_with('-') => bail!("unknown edgerun-aot option: {arg}"),
            _ => positional.push(arg),
        }
    }

    if let Some(first) = positional.first() {
        out.file = first.clone();
    }
    if positional.len() > 1 {
        bail!(
            "unexpected positional arguments: {}",
            positional[1..].join(" ")
        );
    }

    Ok(out)
}

fn next_value<I>(it: &mut std::iter::Peekable<I>, flag: &str) -> Result<String>
where
    I: Iterator<Item = String>,
{
    it.next()
        .filter(|v| !v.is_empty())
        .with_context(|| format!("missing value for {flag}"))
}

fn print_help() {
    println!("edgerun-aot [app.wasm] [options]");
    println!("  --emit-ir");
    println!("  -o, --output <path>");
    println!("  --inspect-artifact <path>");
    println!("  --verify-artifact <path>");
    println!("  --run-artifact <path>");
    println!("  --function <name-or-index>      default: run");
    println!("  --args <a,b,c>                 comma-separated integer args");
    println!("  -v, --verbose");
}

fn compile_artifact(args: &Args) -> Result<()> {
    let input_path = Path::new(&args.file);
    let wasm = std::fs::read(input_path).context("failed to read WASM input")?;

    wasmparser_mock::Validator::new()
        .validate_all(&wasm)
        .context("invalid WASM module")?;

    let module = parse_module(&wasm)?;
    let functions = lower_module(module)?;

    if functions.is_empty() {
        bail!("no baseline-AOT-compilable functions found");
    }

    let module_code = X86_64ModuleBackend::compile_module(&functions)?;
    let mut compiled = Vec::new();
    for meta in &module_code.functions {
        let ir = functions
            .iter()
            .find(|f| f.index == meta.index)
            .with_context(|| format!("missing IR for compiled function {}", meta.index))?
            .clone();
        let start = meta.offset as usize;
        let end = start + meta.len as usize;
        let code = module_code
            .code
            .get(start..end)
            .with_context(|| format!("invalid module code range for function {}", meta.index))?
            .to_vec();
        if args.verbose {
            eprintln!("compiled {}: {} bytes", meta.name, code.len());
        }
        compiled.push(CompiledFunction {
            index: meta.index,
            name: meta.name.clone(),
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

    let output = if args.output.is_empty() {
        input_path.with_extension("eraot")
    } else {
        PathBuf::from(&args.output)
    };
    std::fs::write(&output, artifact.encode()).context("failed to write AOT artifact")?;

    println!("PASS: wrote {}", output.display());
    Ok(())
}

fn decode_verified_artifact(wasm_path: &str, artifact_path: &str) -> Result<DecodedAotArtifact> {
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

    Ok(decoded)
}

fn verify_artifact(wasm_path: &str, artifact_path: &str, verbose: bool) -> Result<()> {
    let decoded = decode_verified_artifact(wasm_path, artifact_path)?;

    if verbose {
        eprintln!("artifact: {}", artifact_path);
        eprintln!("target: {}", decoded.target);
        eprintln!("compiler: {}", decoded.compiler);
        eprintln!("functions: {}", decoded.functions.len());
        for f in &decoded.functions {
            eprintln!(
                "  {} #{} {}: {} bytes",
                f.name,
                f.index,
                f.sig,
                f.code.len()
            );
        }
    }

    println!("PASS: verified {} against {}", artifact_path, wasm_path);
    Ok(())
}

fn run_artifact(
    wasm_path: &str,
    artifact_path: &str,
    function_selector: &str,
    arg_values: &str,
    verbose: bool,
) -> Result<()> {
    let decoded = decode_verified_artifact(wasm_path, artifact_path)?;
    let function = find_function(&decoded, function_selector)?;
    let args = parse_arg_list(arg_values)?;

    if verbose {
        eprintln!("artifact: {}", artifact_path);
        eprintln!(
            "function: {} #{} {}",
            function.name, function.index, function.sig
        );
        eprintln!("args: {:?}", args);
    }

    let loaded = LoadedFunction::from_artifact_function(function)?;
    let result = loaded.call_u64(&args)?;
    println!("{}", result);
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
        println!(
            "    #{} {} {}: {} bytes",
            f.index,
            f.name,
            f.sig,
            f.code.len()
        );
    }

    Ok(())
}

fn parse_arg_list(values: &str) -> Result<Vec<u64>> {
    let trimmed = values.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    trimmed
        .split(',')
        .map(|v| parse_u64_arg(v.trim()))
        .collect()
}

fn parse_u64_arg(value: &str) -> Result<u64> {
    if value.is_empty() {
        bail!("empty argument in --args list");
    }
    if let Some(hex) = value.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16)
            .with_context(|| format!("invalid hex argument: {value}"));
    }
    if let Some(hex) = value.strip_prefix("0X") {
        return u64::from_str_radix(hex, 16)
            .with_context(|| format!("invalid hex argument: {value}"));
    }
    if value.starts_with('-') {
        let signed = value
            .parse::<i64>()
            .with_context(|| format!("invalid signed argument: {value}"))?;
        return Ok(signed as u64);
    }
    value
        .parse::<u64>()
        .with_context(|| format!("invalid integer argument: {value}"))
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
