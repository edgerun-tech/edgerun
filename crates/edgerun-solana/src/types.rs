use crate::prelude::*;
use crate::solana_types::Pubkey;
use edgerun_json::{from_json_slice, FromJson, JsonValue, JsonValueError, ToJson};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderStatus {
    #[default]
    Active = 0,
    Paused = 1,
    Slashed = 2,
}

impl ToJson for ProviderStatus {
    fn to_json(&self) -> JsonValue {
        (*self as u64).to_json()
    }
}

impl FromJson for ProviderStatus {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match u64::from_json(value)? {
            0 => Ok(Self::Active),
            1 => Ok(Self::Paused),
            2 => Ok(Self::Slashed),
            other => Err(JsonValueError::WrongType(format!(
                "unknown provider status {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeploymentStatus {
    #[default]
    Created = 0,
    Running = 1,
    Paused = 2,
    Stopped = 3,
    Disputed = 4,
}

impl ToJson for DeploymentStatus {
    fn to_json(&self) -> JsonValue {
        (*self as u64).to_json()
    }
}

impl FromJson for DeploymentStatus {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        match u64::from_json(value)? {
            0 => Ok(Self::Created),
            1 => Ok(Self::Running),
            2 => Ok(Self::Paused),
            3 => Ok(Self::Stopped),
            4 => Ok(Self::Disputed),
            other => Err(JsonValueError::WrongType(format!(
                "unknown deployment status {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Provider {
    pub authority: Pubkey,
    pub collateral_staked: u64,
    pub cpu_cores: u32,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_mbits: u32,
    pub total_earnings: u64,
    pub slash_count: u32,
    pub uptime_percent: u32,
    pub status: ProviderStatus,
}

edgerun_json::impl_json_struct! {
    Provider {
        authority: Pubkey,
        collateral_staked: u64,
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
        total_earnings: u64,
        slash_count: u32,
        uptime_percent: u32,
        status: ProviderStatus,
    }
}

#[derive(Debug, Clone)]
pub struct Deployment {
    pub owner: Pubkey,
    pub provider: Pubkey,
    pub name: Vec<u8>,
    pub container_count: u32,
    pub total_cpu_cores: u32,
    pub total_memory_bytes: u64,
    pub total_storage_bytes: u64,
    pub total_network_mbps: u32,
    pub deposit: u64,
    pub burn_rate: u64,
    pub spent: u64,
    pub status: DeploymentStatus,
    pub created_at: i64,
    pub started_at: i64,
    pub paused_at: i64,
}

edgerun_json::impl_json_struct! {
    Deployment {
        owner: Pubkey,
        provider: Pubkey,
        name: Vec<u8>,
        container_count: u32,
        total_cpu_cores: u32,
        total_memory_bytes: u64,
        total_storage_bytes: u64,
        total_network_mbps: u32,
        deposit: u64,
        burn_rate: u64,
        spent: u64,
        status: DeploymentStatus,
        created_at: i64,
        started_at: i64,
        paused_at: i64,
    }
}

impl Default for Deployment {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            provider: Pubkey::default(),
            name: Vec::new(),
            container_count: 0,
            total_cpu_cores: 0,
            total_memory_bytes: 0,
            total_storage_bytes: 0,
            total_network_mbps: 0,
            deposit: 0,
            burn_rate: 0,
            spent: 0,
            status: DeploymentStatus::Created,
            created_at: 0,
            started_at: 0,
            paused_at: 0,
        }
    }
}

impl Deployment {
    pub fn try_from_slice(data: &[u8]) -> Result<Self, String> {
        from_json_slice(data).map_err(|e| format!("{:?}", e))
    }
}

impl Provider {
    pub fn try_from_slice(data: &[u8]) -> Result<Self, String> {
        from_json_slice(data).map_err(|e| format!("{:?}", e))
    }
}

pub mod collateral {
    pub const PER_CORE: u64 = 100_000_000;
    pub const PER_GIB_RAM: u64 = 50_000_000;
    pub const PER_GIB_STORAGE: u64 = 10_000_000;
    pub const PER_MBIT: u64 = 5_000_000;

    pub fn calculate_minimum(
        cpu_cores: u32,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    ) -> u64 {
        let ram_gib = memory_bytes.div_ceil(1024 * 1024 * 1024);
        let storage_gib = storage_bytes.div_ceil(1024 * 1024 * 1024);
        PER_CORE * cpu_cores as u64
            + PER_GIB_RAM * ram_gib
            + PER_GIB_STORAGE * storage_gib
            + PER_MBIT * network_mbits as u64
    }
}

pub mod pricing {
    pub const CORE_HOUR: u64 = 10_000;
    pub const RAM_GIB_HOUR: u64 = 5_000;
    pub const STORAGE_GIB_HOUR: u64 = 1_000;
    pub const NETWORK_MBIT_HOUR: u64 = 2_000;
}
