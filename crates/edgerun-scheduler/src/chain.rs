//! On-chain watcher — polls Solana RPC for deployment account changes.

use crate::deployment::DeploymentManager;
use edgerun_solana::{DeploymentClient, solana_types::Pubkey};

pub struct ChainWatcher {
    client: DeploymentClient,
    poll_interval_secs: u64,
}

impl ChainWatcher {
    pub fn new(rpc_url: &str, poll_interval_secs: u64) -> Self {
        Self {
            client: DeploymentClient::new(rpc_url).expect("valid RPC URL"),
            poll_interval_secs,
        }
    }

    pub fn poll(&self, dm: &mut DeploymentManager) {
        for handle in dm.iter_mut().values_mut() {
            let pubkey = Pubkey::new_from_array(handle.on_chain_address);
            if let Ok(deployment) = self.client.get_deployment(&pubkey) {
                handle.status = deployment.status;
                handle.spent = deployment.spent;
                handle.total_cpu_cores = deployment.total_cpu_cores;
                handle.total_memory_bytes = deployment.total_memory_bytes;
            }
        }
    }

    pub fn poll_interval(&self) -> u64 {
        self.poll_interval_secs
    }
}