#[path = "aot/artifact.rs"]
mod artifact;
#[path = "aot/backend_x86_64.rs"]
mod backend_x86_64;
#[path = "aot/ir.rs"]
mod ir;
#[path = "aot/lower.rs"]
mod lower;

use anyhow::{bail, Context, Result};
use artifact::{AotArtifact, CompiledFunction};
use backend_x86_64::X86_64Backend;
use edgerun_clap::Parser;
use lower::{lower_module, parse_module};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "edgerun-aot", about = "Compile a small deterministic WASM subset into an EdgeRun AOT artifact")]
struct Args {
    #[arg(default_value = "app.wasm")]
    file: String,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long)]
    emit_ir: bool,

    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
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
        target: "x86_64-linux-sysv",
        compiler: "edgerun-aot-baseline-v0",
        functions: compiled,
    };

    if args.emit_ir {
        for f in &artifact.functions {
            println!("{}", f.ir.render());
        }
    }

    let output = args
        .output
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.with_extension("eraot"));
    std::fs::write(&output, artifact.encode()).context("failed to write AOT artifact")?;

    println!("PASS: wrote {}", output.display());
    Ok(())
}
