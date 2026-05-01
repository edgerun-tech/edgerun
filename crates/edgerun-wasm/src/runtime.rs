//! EdgeRun Minimal Runtime
//! 
//! Simple runtime for testing EdgeRun WASM modules locally.
//! Uses Wasmtime to execute WASM with injected host functions.

use anyhow::Result;
use clap::Parser;
use std::path::Path;
use wasmtime::*;

/// EdgeRun WASM Runtime
#[derive(Parser, Debug)]
#[command(name = "edgerun-runtime")]
#[command(about = "Run EdgeRun WASM modules locally", long_about = None)]
struct Args {
    /// WASM file to run
    #[arg(default_value = "app.wasm")]
    file: String,
    
    /// Input to pass to run function (JSON string)
    #[arg(short, long, default_value = "")]
    input: String,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let wasm_path = Path::new(&args.file);
    if !wasm_path.exists() {
        anyhow::bail!("File not found: {}", args.file);
    }
    
    // Create engine
    letEngine = Engine::default();
    
    // Load module
    let module = Module::from_file(&engine, wasm_path)
        .context("Failed to load WASM module")?;
    
    if args.verbose {
        eprintln!("Loaded: {}", args.file);
    }
    
    // Create linker with host functions
    let linker = Linker::new(&engine);
    
    // Define host functions in `env` namespace
    linker.func_wrap(
        "env",
        "write_output",
        |caller: Caller<'_>, ptr: i32, len: i32| {
            // Read memory from WASM and print
            let mem = caller.get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("No memory export");
            
            let data = mem.data_ptr();
            unsafe {
                let slice = std::slice::from_raw_parts(data.add(ptr as usize), len as usize);
                println!("OUTPUT: {}", String::from_utf8_lossy(slice));
            }
        },
    )?;
    
    // Create instance
    let instance = linker.instantiate(&module)?
        .ensure_no_start(&module);
    
    let run_func = instance
        .get_typed_func::<(i32, i32), i32>("run")
        .context("Failed to find 'run' function")?;
    
    if args.verbose {
        eprintln!("Found entry point: run(ptr, len)");
    }
    
    // Prepare input
    let input_bytes = args.input.as_bytes();
    let input_ptr = input_bytes.as_ptr() as i32;
    let input_len = input_bytes.len() as i32;
    
    if args.verbose {
        eprintln!("Input: {} bytes", input_len);
    }
    
    // Call run function
    let result = run_func.call((input_ptr, input_len))?;
    
    println!("Result: {}", result);
    
    Ok(())
}