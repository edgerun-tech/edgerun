//! AST-level Rust code editor HTTP server.
//!
//! POST /edit — batch edit Rust files with automatic git rollback.
//!
//! Never use sed, grep, heredocs, or string replacement on Rust files.
//! All operations are AST-level: parse → transform → prettyplease → write.

use std::path::PathBuf;

use clap::Parser;
use edgerun_edit::start_server;

#[derive(Parser)]
#[command(name = "edgerun-edit", about = "AST-level Rust code editor HTTP server")]
struct Cli {
    #[arg(long, short = 'p', default_value = "3456")]
    port: u16,
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
}

fn main() {
    let cli = Cli::parse();
    let addr = format!("{}:{}", cli.host, cli.port);
    let rt = edgerun_rt::Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        match start_server(&addr).await {
            Ok(server) => {
                eprintln!("edgerun-edit server listening on {}", server.local_addr());
                eprintln!("POST /edit — edit operations");
                eprintln!("GET  /health — health check");
                eprintln!("POST /health — shutdown");
                server.serve().await;
            }
            Err(e) => {
                eprintln!("Failed to start server: {e}");
                std::process::exit(1);
            }
        }
    });
}
