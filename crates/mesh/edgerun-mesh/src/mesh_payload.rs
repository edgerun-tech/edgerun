//! JSON-serializable payload types for mesh control frames.

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct DeploymentMetrics {
    pub name: Vec<u8>,
    pub cpu_cores_used: u32,
    pub memory_bytes_used: u64,
    pub network_bytes_sent: u64,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct MigrationCompletePayload {
    pub deployment_name: Vec<u8>,
    pub success: bool,
    pub error_message: Option<String>,
}

edgerun_json::impl_json_struct! {
    MetricsReportPayload {
        timestamp: u64,
        cpu_cores_used: u32,
        cpu_cores_available: u32,
        memory_bytes_used: u64,
        memory_bytes_available: u64,
        storage_bytes_used: u64,
        storage_bytes_available: u64,
        network_bytes_sent: u64,
        network_bytes_received: u64,
        container_count: u32,
        deployments: Vec<DeploymentMetrics>,
    }
}

edgerun_json::impl_json_struct! {
    DeploymentMetrics {
        name: Vec<u8>,
        cpu_cores_used: u32,
        memory_bytes_used: u64,
        network_bytes_sent: u64,
    }
}

edgerun_json::impl_json_struct! {
    MigrationOrderPayload {
        deployment_name: Vec<u8>,
        container_image: String,
        container_env: Vec<(String, String)>,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        replicas: u32,
        source_provider: Vec<u8>,
    }
}

edgerun_json::impl_json_struct! {
    MigrationCompletePayload {
        required {
            deployment_name: "deployment_name" => Vec<u8>,
            success: "success" => bool,
        }
        optional {
            error_message: "error_message" => String,
        }
    }
}
use crate::prelude::v1::*;
