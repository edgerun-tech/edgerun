mod bundle;
mod validate;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{bundle::decode_bundle_from_canonical_bytes, validate::validate_wasm_module};

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { bundle, output } => run(bundle, output).await?,
    }

    Ok(())
}

async fn run(bundle_path: PathBuf, output_path: PathBuf) -> Result<()> {
    let bundle_bytes = tokio::fs::read(&bundle_path).await?;
    let bundle_hash = edgerun_crypto::compute_bundle_hash(&bundle_bytes);
    let bundle = decode_bundle_from_canonical_bytes(&bundle_bytes)?;

    // Hash first, decode second. Never hash decoded/re-encoded payload.
    validate_wasm_module(&bundle.wasm)?;

    // TODO: execute deterministic host environment and write actual output.
    tokio::fs::write(&output_path, []).await?;

    println!("bundle_hash={}", hex::encode(bundle_hash));
    println!("output_hash={}", hex::encode(edgerun_crypto::blake3_256(&[])));
    println!("input_len={}", bundle.input.len());
    println!("max_memory_bytes={}", bundle.limits.max_memory_bytes);
    println!("max_instructions={}", bundle.limits.max_instructions);

    Ok(())
}
