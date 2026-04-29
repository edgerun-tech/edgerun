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
    let key = match bytes.len() {
        32 => {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            key
        }
        _ => parse_solana_cli_keypair_json(&bytes)?,
    };
    Ok(edgerun_solana::signers::Ed25519Signer::from_bytes(&key))
}

#[cfg(not(target_os = "none"))]
fn parse_solana_cli_keypair_json(bytes: &[u8]) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let text = std::str::from_utf8(bytes)?;
    let parsed = edgerun_json::parse_json(text)?;
    let values = parsed
        .as_array()
        .ok_or("Solana CLI keypair must be a JSON byte array")?;
    if values.len() != 64 {
        return Err("Solana CLI keypair JSON must contain 64 bytes".into());
    }

    let mut key = [0u8; 32];
    for (idx, value) in values.iter().take(32).enumerate() {
        let byte = value
            .as_u64()
            .and_then(|n| u8::try_from(n).ok())
            .ok_or("Solana CLI keypair contains a non-byte value")?;
        key[idx] = byte;
    }
    Ok(key)
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
            .map(std::path::PathBuf::from)
            .or_else(default_solana_keypair_path);

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
fn default_solana_keypair_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home)
        .join(".config")
        .join("solana")
        .join("id.json");
    path.exists().then_some(path)
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::vec::Vec;

    #[test]
    fn parses_solana_cli_keypair_json_seed_prefix() {
        let bytes = (0u8..64)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let json = format!("[{}]", bytes);
        let seed = parse_solana_cli_keypair_json(json.as_bytes()).unwrap();
        assert_eq!(seed, core::array::from_fn::<_, 32, _>(|idx| idx as u8));
    }

    #[test]
    fn rejects_solana_cli_keypair_with_wrong_length() {
        let err = parse_solana_cli_keypair_json(b"[1,2,3]").unwrap_err();
        assert!(err.to_string().contains("64 bytes"));
    }
}

#[cfg(not(target_os = "none"))]
fn print_usage() {
    eprintln!("Usage: edgerun-marketplace <command> [options]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!(
        "  provider    Provider operations (register, get, list, earnings, attest, pause, resume)"
    );
    eprintln!(
        "  deployment  Deployment operations (create, get, start, pause, resume, dispute, resolve, stop, tick-burn, report, burn-rate, schedule-pricing)"
    );
    eprintln!("  status      Show marketplace status");
    eprintln!();
    eprintln!("Environment variables:");
    eprintln!("  SOLANA_RPC_URL   Solana RPC URL (default: https://api.devnet.solana.com)");
    eprintln!("  SOLANA_KEYPAIR   Path to keypair file (Solana JSON or 32-byte raw key)");
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

    let rt = edgerun_rt::Builder::new_multi_thread().build()?;

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
