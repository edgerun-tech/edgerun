//! Handler for incoming mesh metrics frames.
//!
//! Processes MetricsReport frames and reports to on-chain.

use edgerun_json::from_slice;
use edgerun_mesh::FrameType;
use edgerun_mesh::mesh_payload::MetricsReportPayload;
use edgerun_solana::DeploymentClient;
use edgerun_solana::signers::Ed25519Signer;
use crate::Scheduler;

impl Scheduler {
    #[allow(unused_variables)]
    /// Process a mesh frame and handle metrics reporting.
    pub fn handle_mesh_frame(
        &mut self,
        frame_type: FrameType,
        src: [u8; 32],
        payload: &[u8],
        signer: Option<&Ed25519Signer>,
        deployment_client: Option<&DeploymentClient>,
    ) -> Result<(), String> {
        match frame_type {
            FrameType::MetricsReport => {
                self.handle_metrics_report(src, payload)?;
            }
            FrameType::MigrationComplete => {
                self.handle_migration_complete(src, payload)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_metrics_report(
        &mut self,
        src: [u8; 32],
        payload: &[u8],
    ) -> Result<(), String> {
        let report: MetricsReportPayload = from_slice(payload)
            .map_err(|e| format!("Failed to deserialize metrics: {}", e))?;
        
        let metrics = self.metrics_receiver.receive_from_mesh(src, &report)?;
        
        if let Some(p) = self.provider_manager.get_mut(&src) {
            p.cpu_cores_used = metrics.cpu_cores_used;
            p.memory_bytes_used = metrics.memory_bytes_used;
            p.is_online = true;
        }
        
        Ok(())
    }

    fn handle_migration_complete(
        &mut self,
        _src: [u8; 32],
        payload: &[u8],
    ) -> Result<(), String> {
        use edgerun_mesh::mesh_payload::MigrationCompletePayload;
        
        let result: MigrationCompletePayload = from_slice(payload)
            .map_err(|e| format!("Failed to deserialize migration result: {}", e))?;
        
        let name = String::from_utf8_lossy(&result.deployment_name);
        
        if result.success {
            self.deployment_manager.mark_running(&name);
        } else {
            let err = result.error_message.as_deref().unwrap_or("Unknown error");
            self.deployment_manager.set_error(&name, err);
        }
        
        Ok(())
    }
}