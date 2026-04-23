//! CLI configuration and keypair loading utilities.

use std::path::PathBuf;

pub struct Config {
    pub rpc_url: String,
    pub keypair_path: Option<PathBuf>,
}

impl Config {
    pub fn load() -> Self {
        Self {
            rpc_url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            keypair_path: std::env::var("SOLANA_KEYPAIR")
                .ok()
                .map(PathBuf::from),
        }
    }
}