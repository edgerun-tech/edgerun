//! Marketplace CLI — interact with Edgerun on-chain programs via Solana RPC.

mod provider;
mod deployment;

pub struct Cli {
    pub rpc_url: String,
    pub keypair: Option<std::path::PathBuf>,
    pub command: Command,
}

pub enum Command {
    Provider(provider::ProviderCommand),
    Deployment(deployment::DeploymentCommand),
    Status,
}

impl Cli {
    pub fn parse() -> Self {
        let mut args = std::env::args();
        let _ = args.next();
        
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
        
        let keypair = std::env::var("SOLANA_KEYPAIR").ok().map(std::path::PathBuf::from);
        
        let cmd = match args.next().as_deref() {
            Some("provider") | Some("p") => {
                Command::Provider(provider::parse_provider_command())
            }
            Some("deployment") | Some("d") => {
                Command::Deployment(deployment::parse_deployment_command())
            }
            Some("status") => Command::Status,
            _ => {
                print_usage();
                std::process::exit(1);
            }
        };
        
        Self {
            rpc_url,
            keypair,
            command: cmd,
        }
    }
}

fn print_usage() {
    eprintln!("Usage: edgerun-marketplace <command> [options]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  provider    Provider operations (register, get, list, attest, pause, resume)");
    eprintln!("  deployment  Deployment operations (create, get, start, stop, report, burn-rate)");
    eprintln!("  status      Show marketplace status");
    eprintln!();
    eprintln!("Environment variables:");
    eprintln!("  SOLANA_RPC_URL   Solana RPC URL (default: https://api.devnet.solana.com)");
    eprintln!("  SOLANA_KEYPAIR   Path to keypair file");
}

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    
    let rt = edgerun_rt::Builder::new_multi_thread().build()?;
    
    match cli.command {
Command::Provider(cmd) => {
            let rpc_url = cli.rpc_url.clone();
            rt.block_on(provider::handle(cmd, rpc_url))
        }
        Command::Deployment(cmd) => {
            let rpc_url = cli.rpc_url.clone();
            rt.block_on(deployment::handle(cmd, rpc_url))
        }
        Command::Deployment(cmd) => {
            let rpc_url = cli.rpc_url.clone();
            rt.block_on(deployment::handle(cmd, rpc_url))
        }
        Command::Status => {
            println!("=== Edgerun Marketplace Status ===");
            println!("RPC: {}", cli.rpc_url);
            Ok(())
        }
    }
}