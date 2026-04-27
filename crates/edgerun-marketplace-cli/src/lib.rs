//! Marketplace CLI — interact with Edgerun on-chain programs via Solana RPC.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(not(target_os = "none"))]
use alloc::boxed::Box;
use alloc::string::String;
#[cfg(not(target_os = "none"))]
use alloc::string::ToString;
#[cfg(target_os = "none")]
use core::fmt;
#[cfg(not(target_os = "none"))]
use std::{eprintln, println};

#[cfg(not(target_os = "none"))]
mod deployment;
#[cfg(not(target_os = "none"))]
mod provider;

#[cfg(target_os = "none")]
mod deployment {
    pub enum DeploymentCommand {}
}

#[cfg(target_os = "none")]
mod provider {
    pub enum ProviderCommand {}
}

pub struct Cli {
    pub rpc_url: String,
    #[cfg(not(target_os = "none"))]
    pub keypair_path: Option<std::path::PathBuf>,
    pub command: Command,
}

pub enum Command {
    Provider(provider::ProviderCommand),
    Deployment(deployment::DeploymentCommand),
    Status,
}

#[cfg(not(target_os = "none"))]
pub fn load_keypair(
    path: &std::path::Path,
) -> Result<edgerun_solana::signers::Ed25519Signer, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    if bytes.len() != 32 {
        return Err("Keypair must be 32 bytes".into());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(edgerun_solana::signers::Ed25519Signer::from_bytes(&key))
}

#[cfg(not(target_os = "none"))]
impl Cli {
    pub fn parse() -> Self {
        let mut args = std::env::args();
        let _ = args.next();

        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

        let keypair_path = std::env::var("SOLANA_KEYPAIR")
            .ok()
            .map(std::path::PathBuf::from);

        let cmd = match args.next().as_deref() {
            Some("provider") | Some("p") => Command::Provider(provider::parse_provider_command()),
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
            keypair_path,
            command: cmd,
        }
    }
}

#[cfg(not(target_os = "none"))]
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
    eprintln!("  SOLANA_KEYPAIR   Path to keypair file (32-byte raw key)");
}

#[cfg(not(target_os = "none"))]
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    let signer = match &cli.keypair_path {
        Some(path) => match load_keypair(path) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!(
                    "Warning: Failed to load keypair: {}. Transactions will not be signed.",
                    e
                );
                None
            }
        },
        None => None,
    };

    let rt = edgerun_bare_rt::Builder::new_multi_thread().build()?;

    match cli.command {
        Command::Provider(cmd) => {
            let rpc_url = cli.rpc_url.clone();
            rt.block_on(provider::handle(cmd, rpc_url, signer))
        }
        Command::Deployment(cmd) => {
            let rpc_url = cli.rpc_url.clone();
            rt.block_on(deployment::handle(cmd, rpc_url, signer))
        }
        Command::Status => {
            println!("=== Edgerun Marketplace Status ===");
            println!("RPC: {}", cli.rpc_url);
            if cli.keypair_path.is_some() {
                println!("Keypair: loaded");
            } else {
                println!("Keypair: not configured (transactions will be unsigned)");
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "none")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketplaceCliUnavailable;

#[cfg(target_os = "none")]
impl fmt::Display for MarketplaceCliUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("edgerun-marketplace CLI is unavailable on bare target")
    }
}

#[cfg(target_os = "none")]
impl core::error::Error for MarketplaceCliUnavailable {}

#[cfg(target_os = "none")]
pub fn run() -> Result<(), MarketplaceCliUnavailable> {
    Err(MarketplaceCliUnavailable)
}
