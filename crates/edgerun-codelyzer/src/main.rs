mod analyzer;
mod diagnostics;
#[allow(dead_code)]
mod edit;
mod filesystem;
mod git;
mod parser;
mod repo_registry;
mod tools;
mod uir;

use std::process;

fn print_usage(prog: &str) {
    println!("Usage: {} <path>", prog);
    println!();
    println!("Multi-language code analysis engine");
    println!();
    println!("Options:");
    println!("  -h, --help           Show this help");
    println!();
    println!("Run the indexer to build the codebase database:");
    println!("  edgerun-codelyzer-index <path>");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut path: Option<&str> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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

    if !std::path::Path::new(path).is_dir() {
        eprintln!("Error: '{}' is not a directory", path);
        process::exit(1);
    }

    println!("Analyzing codebase at: {}", path);
    println!("Use 'edgerun-codelyzer-index {}' to build the index database.", path);
}
