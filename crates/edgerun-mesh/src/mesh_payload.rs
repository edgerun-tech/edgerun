//! Serde-serializable payload types for mesh control frames.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsReportPayload {
    pub timestamp: u64,
    pub cpu_cores_used: u32,
    pub cpu_cores_available: u32,
    pub memory_bytes_used: u64,
    pub memory_bytes_available: u64,
    pub storage_bytes_used: u64,
    pub storage_bytes_available: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub container_count: u32,
    pub deployments: Vec<DeploymentMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetrics {
    pub name: Vec<u8>,
    pub cpu_cores_used: u32,
    pub memory_bytes_used: u64,
    pub network_bytes_sent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationOrderPayload {
    pub deployment_name: Vec<u8>,
    pub container_image: String,
    pub container_env: Vec<(String, String)>,
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub replicas: u32,
    pub source_provider: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCompletePayload {
    pub deployment_name: Vec<u8>,
    pub success: bool,
    pub error_message: Option<String>,
}
use crate::prelude::v1::*;
