//! Binary entry point for the edgerun-agent coding assistant.

use edgerun_config::yaml;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Config {
    server: ServerConfig,
    tabby: TabbyConfig,
    agent: AgentConfig,
}

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Deserialize)]
struct TabbyConfig {
    url: String,
    model: String,
}

#[derive(Deserialize)]
struct AgentConfig {
    project: String,
    timeout_secs: u64,
    temperature: f32,
    max_tokens: u32,
    commands: Vec<String>,
}

fn load_config() -> Option<Config> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path = manifest.join("config.yaml");
    
    if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                match yaml::from_str(&content) {
                    Ok(config) => {
                        println!("Loaded config from {:?}", config_path);
                        Some(config)
                    }
                    Err(e) => {
                        eprintln!("Failed to parse config.yaml: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read config.yaml: {}", e);
                None
            }
        }
    } else {
        println!("No config.yaml found at {:?}", config_path);
        None
    }
}

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
    #[arg(long)]
    temperature: Option<f32>,
    #[arg(long)]
    max_tokens: Option<u32>,
}

fn main() {
    let cli = Cli::parse();
    let config = load_config();

    let (host, port, tabby_url, model, project, timeout_secs, commands, temperature, max_tokens) = 
        if let Some(cfg) = &config {
            let use_cli_args = cli.port != 8080 || !cli.tabby_url.is_empty() && cli.tabby_url != "http://10.10.10.1:5001";
            (
                if use_cli_args { cli.host.clone() } else { cfg.server.host.clone() },
                if cli.port != 8080 { cli.port } else { cfg.server.port },
                if !cli.tabby_url.is_empty() && cli.tabby_url != "http://10.10.10.1:5001" { cli.tabby_url } else { cfg.tabby.url.clone() },
                if !cli.model.is_empty() && cli.model != "devstral-small-2:24b" { cli.model } else { cfg.tabby.model.clone() },
                if cli.project != "." { cli.project.clone() } else { cfg.agent.project.clone() },
                if cli.timeout_secs != 30 { cli.timeout_secs } else { cfg.agent.timeout_secs },
                cli.commands.clone().unwrap_or_else(|| cfg.agent.commands.clone()),
                cli.temperature.or(Some(cfg.agent.temperature)),
                cli.max_tokens.or(Some(cfg.agent.max_tokens)),
            )
        } else {
            (
                cli.host,
                cli.port,
                cli.tabby_url,
                cli.model,
                cli.project,
                cli.timeout_secs,
                cli.commands.unwrap_or_else(|| {
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
                }),
                cli.temperature,
                cli.max_tokens,
            )
        };

    let static_dir = cli.static_dir.unwrap_or_else(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest.join("ui").to_string_lossy().to_string()
    });

    let addr = format!("{}:{}", host, port);

    println!("edgerun-agent starting...");
    println!("  TabbyAPI: {}", tabby_url);
    println!("  Model:    {}", model);
    println!("  Project:  {}", std::fs::canonicalize(&project).unwrap_or_else(|_| project.clone().into()).display());
    println!("  Static:   {}", static_dir);
    println!("  Address:  {}", addr);
    println!("  Timeout:  {}s per command", timeout_secs);
    if let Some(t) = temperature {
        println!("  Temperature: {}", t);
    }
    if let Some(m) = max_tokens {
        println!("  Max tokens: {}", m);
    }
    println!("  Commands: {}", commands.join(", "));

    let web_server = edgerun_agent::web::WebServer::new(
        &static_dir,
        &project,
        &tabby_url,
        &model,
    )
    .with_allowed_commands(commands)
    .with_temperature(temperature)
    .with_max_tokens(max_tokens);
    let handler = web_server.into_handler();

    let rt = edgerun_rt::Builder::new_multi_thread()
        .worker_threads(4)
        .max_blocking_threads(8)
        .build()
        .expect("Failed to create runtime");

    rt.block_on(async move {
        let shutdown = edgerun_rt::CancellationToken::new();

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
