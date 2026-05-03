mod analyzer;
mod diagnostics;
#[allow(dead_code)]
mod edit;
mod filesystem;
mod git;
mod parser;
mod qwen;
mod repo_registry;
#[cfg(feature = "server")]
mod server_http;
mod tools;
mod uir;

use std::{env, path::Path, process};

#[cfg(feature = "server")]
use edgerun_http::HttpServer;
#[cfg(feature = "server")]
use server_http::CodelyzerHandler;

fn print_usage(prog: &str) {
    println!("Usage: {} [OPTIONS] <path>", prog);
    println!();
    println!("Multi-language code analysis engine with WebSocket viewer");
    println!();
    println!("Options:");
    println!("  -p, --port <port>    Server port (default: 13337)");
    println!("  -i, --incremental    Incremental analysis (unused with server mode)");
    println!("  -h, --help           Show this help");
    println!();
    println!("Endpoints:");
    println!("  GET  /              → viewer");
    println!("  WS   /ws            → graph data + debug logs");
    println!("  POST /api/chat      → chat with Qwen");
}

#[cfg(feature = "server")]
fn main() {
    let args: Vec<String> = env::args().collect();

    let mut path: Option<&str> = None;
    let mut port: u16 = 13337;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                i += 1;
                if i < args.len() {
                    port = args[i].parse().unwrap_or(8080);
                }
            }
            "--incremental" | "-i" => {}
            "--help" | "-h" => {
                print_usage(&args[0]);
                return;
            }
            _ => path = Some(&args[i]),
        }
        i += 1;
    }

    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("Error: no path specified");
            print_usage(&args[0]);
            process::exit(1);
        }
    };

    if !Path::new(path).is_dir() {
        eprintln!("Error: '{}' is not a directory", path);
        process::exit(1);
    }

    let viewer_dir = format!("{}/viewer", env!("CARGO_MANIFEST_DIR"));
    if !Path::new(&viewer_dir).is_dir() {
        eprintln!("Error: viewer directory not found at {}", viewer_dir);
        process::exit(1);
    }

    // Initialize app state
    let state = server_http::AppState::new(path);
    *server_http::APP_STATE.lock().unwrap() = Some(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("[main] Starting server on {}", addr);
    println!("[main] Viewer directory: {}", viewer_dir);

    let rt = edgerun_rt::Runtime::new_multi_thread().enable_all().build().unwrap();
    
    rt.block_on(async {
        let handler = CodelyzerHandler { viewer_dir };
        
        HttpServer::new(handler)
            .bind(addr)
            .await
            .unwrap()
            .serve()
            .await
            .unwrap();
    });
}

#[cfg(not(feature = "server"))]
fn main() {
    eprintln!("This binary requires the 'server' feature to be enabled.");
    eprintln!("Build with: cargo build --features server");
    process::exit(1);
}
