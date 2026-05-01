//! EdgeRun WASM Validator
//! 
//! Validates WASM modules for EdgeRun conformance.

use anyhow::{Context, Result};
use clap::Parser;
use std::path::Path;
use wasmparser::Validator;

#[derive(Parser, Debug)]
#[command(name = "edgerun-validate")]
struct Args {
    #[arg(default_value = "app.wasm")]
    file: String,
    
    #[arg(short, long)]
    max_size: Option<usize>,
    
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let path = Path::new(&args.file);
    let file_size = path.metadata()
        .context("Failed to read file")?
        .len() as usize;
    
    let max_size = args.max_size.unwrap_or(1024 * 1024);
    
    if file_size > max_size {
        anyhow::bail!("File too large: {} bytes (max: {})", file_size, max_size);
    }
    
    if args.verbose {
        eprintln!("Checking: {} ({} bytes)", args.file, file_size);
    }
    
    let wasm = std::fs::read(path).context("Failed to read WASM")?;
    
    // Validate WASM structure
    let mut validator = Validator::new();
    validator.validate_all(&wasm).context("Invalid WASM structure")?;
    
    if args.verbose {
        eprintln!("✓ Valid WASM structure");
    }
    
    // Check for "run" export
    let mut has_run = false;
    for item in wasmparser::Parser::new(0).parse_all(&wasm) {
        if let wasmparser::Payload::ExportSection(s) = item? {
            for e in s {
                if let Ok(export) = e {
                    if export.name == "run" {
                        has_run = true;
                        if args.verbose {
                            eprintln!("✓ Found 'run' export");
                        }
                    }
                }
            }
        }
    }
    
    if !has_run {
        anyhow::bail!("Missing required export: 'run(ptr: i32, len: i32) -> i32'");
    }
    
    println!("✓ {}", args.file);
    println!("  Size: {} bytes (max: {})", file_size, max_size);
    println!("  Valid WASM structure");
    println!("  Has 'run' export");
    
    Ok(())
}