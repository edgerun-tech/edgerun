//! Binary entry point for the edgerun-agent coding assistant.

use clap::Parser;

#[derive(Parser)]
#[command(name = "edgerun-agent", about = "AI-powered coding agent with shell access")]
struct Cli {
    #[arg(long, short = 'p', default_value = "8080")]
    port: u16,
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long, default_value = ".")]
    project: String,
    #[arg(long, default_value = "http://10.10.10.1:5001")]
    tabby_url: String,
    #[arg(long, default_value = "devstral-small-2:24b")]
    model: String,
    #[arg(long)]
    static_dir: Option<String>,
    #[arg(long, value_delimiter = ',')]
    commands: Option<Vec<String>>,
    #[arg(long, default_value = "30")]
    timeout_secs: u64,
}

fn main() {
    let cli = Cli::parse();

    let static_dir = cli.static_dir.unwrap_or_else(|| {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest.join("ui").to_string_lossy().to_string()
    });

    let addr = format!("{}:{}", cli.host, cli.port);

    println!("edgerun-agent starting...");
    println!("  TabbyAPI: {}", cli.tabby_url);
    println!("  Model:    {}", cli.model);
    println!("  Project:  {}", std::fs::canonicalize(&cli.project).unwrap_or_else(|_| cli.project.clone().into()).display());
    println!("  Static:   {}", static_dir);
    println!("  Address:  {}", addr);
    println!("  Timeout:  {}s per command", cli.timeout_secs);

    let commands = cli.commands.unwrap_or_else(|| {
        [
            "cat", "head", "tail", "ls", "find", "grep", "rg", "wc",
            "cargo", "rustc", "rustfmt", "clippy-driver",
            "git", "diff", "echo", "mkdir", "cp", "mv",
            "curl", "jq", "sort", "uniq", "awk", "sed",
            "tree", "which", "env", "pwd", "test",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    });

    println!("  Commands: {}", commands.join(", "));

    let web_server = edgerun_agent::web::WebServer::new(
        &static_dir,
        &cli.project,
        &cli.tabby_url,
        &cli.model,
    )
    .with_allowed_commands(commands);
    let handler = web_server.into_handler();

    let rt = edgerun_rt::Builder::new_multi_thread()
        .worker_threads(4)
        .max_blocking_threads(8)
        .build()
        .expect("Failed to create runtime");

    rt.block_on(async move {
        let shutdown = edgerun_rt::CancellationToken::new();

        // Monitor for shutdown signals (SIGTERM, SIGHUP, SIGQUIT)
        let shutdown_signal = shutdown.clone();
        let _signal_monitor = edgerun_rt::spawn(async move {
            use edgerun_rt::{Signal, SignalKind, yieldnow};

            let sigterm = Signal::new(SignalKind::terminate());
            let sighup = Signal::new(SignalKind::hangup());
            let sigquit = Signal::new(SignalKind::quit());

            if let (Ok(mut t), Ok(mut h), Ok(mut q)) = (sigterm, sighup, sigquit) {
                loop {
                    yieldnow().await;
                    if t.recv().await.is_ok() {
                        println!("\nSIGTERM received, shutting down...");
                        shutdown_signal.cancel();
                        return;
                    }
                    if h.recv().await.is_ok() {
                        println!("\nSIGHUP received, shutting down...");
                        shutdown_signal.cancel();
                        return;
                    }
                    if q.recv().await.is_ok() {
                        println!("\nSIGQUIT received, shutting down...");
                        shutdown_signal.cancel();
                        return;
                    }
                }
            }
        });

        let server = edgerun_server::Server::new().with_http(handler, &addr);

        match server.build().await {
            Ok(mut bound) => {
                println!("Agent server listening on http://{}", addr);
                if let Err(e) = bound.run(shutdown).await {
                    eprintln!("Server error: {}", e);
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Failed to bind to {}: {}", addr, e);
                std::process::exit(1);
            }
        }
    });

    println!("Shutdown complete.");
}
