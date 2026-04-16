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
    println!("  Project:  {}", cli.project);
    println!("  Static:   {}", static_dir);
    println!("  Address:  {}", addr);

    let commands = cli.commands.unwrap_or_else(|| {
        [
            "cat", "head", "tail", "ls", "find", "grep", "rg", "wc",
            "cargo", "rustc", "rustfmt", "clippy-driver",
            "git", "diff", "echo", "mkdir", "cp", "mv",
            "curl", "jq", "sort", "uniq", "awk", "sed",
            "tree", "which", "env", "pwd",
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

    let rt = edgerun_rt::Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let shutdown = edgerun_rt::CancellationToken::new();

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
}