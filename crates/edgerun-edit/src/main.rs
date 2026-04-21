//! AST-level Rust code editor HTTP server.
//!
//! POST /edit — batch edit Rust files with automatic git rollback.
//!
//! Never use sed, grep, heredocs, or string replacement on Rust files.
//! All operations are AST-level: parse → transform → prettyplease → write.

use std::path::PathBuf;

use edgerun_clap::{Arg, Command, Parser};
use edgerun_edit::start_server;

struct Cli {
    port: u16,
    host: String,
}

impl Parser for Cli {
    fn command() -> Command {
        Command::new("edgerun-edit")
            .about("AST-level Rust code editor HTTP server")
            .arg(Arg::new("port").long("port").short('p').default_value("3456"))
            .arg(Arg::new("host").long("host").default_value("127.0.0.1"))
    }

    fn from(matches: &edgerun_clap::ArgMatches) -> Self {
        Self {
            port: matches.get_one::<u16>("port").unwrap_or(3456),
            host: matches.get_one::<String>("host").unwrap_or_else(|| "127.0.0.1".to_string()),
        }
    }
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
