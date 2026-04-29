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
        const MIN_BINARY_LEN: usize = 242;
        if data.len() >= MIN_BINARY_LEN {
            let mut offset = 0usize;
            let take = |offset: &mut usize, len: usize| -> Result<&[u8], String> {
                let end = offset.saturating_add(len);
                let bytes = data
                    .get(*offset..end)
                    .ok_or_else(|| "deployment data too small".to_string())?;
                *offset = end;
                Ok(bytes)
            };
            let read_pubkey = |offset: &mut usize| -> Result<Pubkey, String> {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(take(offset, 32)?);
                Ok(Pubkey::new_from_array(bytes))
            };
            let read_i64 = |offset: &mut usize| -> Result<i64, String> {
                let bytes: [u8; 8] = take(offset, 8)?
                    .try_into()
                    .map_err(|_| "invalid i64 field".to_string())?;
                Ok(i64::from_le_bytes(bytes))
            };
            let read_u32 = |offset: &mut usize| -> Result<u32, String> {
                let bytes: [u8; 4] = take(offset, 4)?
                    .try_into()
                    .map_err(|_| "invalid u32 field".to_string())?;
                Ok(u32::from_le_bytes(bytes))
            };
            let read_u64 = |offset: &mut usize| -> Result<u64, String> {
                let bytes: [u8; 8] = take(offset, 8)?
                    .try_into()
                    .map_err(|_| "invalid u64 field".to_string())?;
                Ok(u64::from_le_bytes(bytes))
            };

            let owner = read_pubkey(&mut offset)?;
            let provider = read_pubkey(&mut offset)?;
            let name = take(&mut offset, 64)?.to_vec();
            let container_count = read_u32(&mut offset)?;
            let total_cpu_cores = read_u32(&mut offset)?;
            let total_memory_bytes = read_u64(&mut offset)?;
            let total_storage_bytes = read_u64(&mut offset)?;
            let total_network_mbps = read_u32(&mut offset)?;
            let deposit = read_u64(&mut offset)?;
            let burn_rate = read_u64(&mut offset)?;
            let spent = read_u64(&mut offset)?;
            let status_byte = take(&mut offset, 1)?[0];
            let status = match status_byte {
                0 => DeploymentStatus::Created,
                1 => DeploymentStatus::Running,
                2 => DeploymentStatus::Paused,
                3 => DeploymentStatus::Stopped,
                4 => DeploymentStatus::Disputed,
                other => return Err(format!("unknown deployment status {other}")),
            };
            let created_at = read_i64(&mut offset)?;
            let started_at = read_i64(&mut offset)?;
            let paused_at = read_i64(&mut offset)?;

            let _bump_seed = take(&mut offset, 1)?;
            let _last_report_at = read_i64(&mut offset)?;
            let _cpu_cores_used = read_u32(&mut offset)?;
            let _memory_bytes_used = read_u64(&mut offset)?;
            let _storage_bytes_used = read_u64(&mut offset)?;
            let _network_bytes_sent = read_u64(&mut offset)?;

            return Ok(Self {
                owner,
                provider,
                name,
                container_count,
                total_cpu_cores,
                total_memory_bytes,
                total_storage_bytes,
                total_network_mbps,
                deposit,
                burn_rate,
                spent,
                status,
                created_at,
                started_at,
                paused_at,
            });
        }

        from_json_slice(data).map_err(|e| format!("{:?}", e))
    }
}

impl Provider {
    pub fn try_from_slice(data: &[u8]) -> Result<Self, String> {
        if data.len() >= 128 {
            let mut authority = [0u8; 32];
            authority.copy_from_slice(&data[0..32]);

            let read_u32 = |range: core::ops::Range<usize>| -> Result<u32, String> {
                let bytes: [u8; 4] = data[range]
                    .try_into()
                    .map_err(|_| "invalid u32 field".to_string())?;
                Ok(u32::from_le_bytes(bytes))
            };
            let read_u64 = |range: core::ops::Range<usize>| -> Result<u64, String> {
                let bytes: [u8; 8] = data[range]
                    .try_into()
                    .map_err(|_| "invalid u64 field".to_string())?;
                Ok(u64::from_le_bytes(bytes))
            };

            let status = match data[80] {
                0 => ProviderStatus::Active,
                1 => ProviderStatus::Paused,
                2 => ProviderStatus::Slashed,
                other => return Err(format!("unknown provider status {other}")),
            };

            return Ok(Self {
                authority: Pubkey::new_from_array(authority),
                collateral_staked: read_u64(32..40)?,
                cpu_cores: read_u32(40..44)?,
                memory_bytes: read_u64(44..52)?,
                storage_bytes: read_u64(52..60)?,
                network_mbits: read_u32(60..64)?,
                total_earnings: read_u64(64..72)?,
                slash_count: read_u32(72..76)?,
                uptime_percent: read_u32(76..80)?,
                status,
            });
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_decodes_program_binary_layout() {
        let mut data = [0u8; 128];
        data[0..32].copy_from_slice(&[7u8; 32]);
        data[32..40].copy_from_slice(&1234u64.to_le_bytes());
        data[40..44].copy_from_slice(&4u32.to_le_bytes());
        data[44..52].copy_from_slice(&8_000u64.to_le_bytes());
        data[52..60].copy_from_slice(&16_000u64.to_le_bytes());
        data[60..64].copy_from_slice(&100u32.to_le_bytes());
        data[64..72].copy_from_slice(&55u64.to_le_bytes());
        data[72..76].copy_from_slice(&2u32.to_le_bytes());
        data[76..80].copy_from_slice(&9900u32.to_le_bytes());
        data[80] = ProviderStatus::Paused as u8;

        let provider = Provider::try_from_slice(&data).unwrap();
        assert_eq!(provider.authority, Pubkey::new_from_array([7u8; 32]));
        assert_eq!(provider.collateral_staked, 1234);
        assert_eq!(provider.cpu_cores, 4);
        assert_eq!(provider.memory_bytes, 8_000);
        assert_eq!(provider.storage_bytes, 16_000);
        assert_eq!(provider.network_mbits, 100);
        assert_eq!(provider.total_earnings, 55);
        assert_eq!(provider.slash_count, 2);
        assert_eq!(provider.uptime_percent, 9900);
        assert_eq!(provider.status, ProviderStatus::Paused);
    }

    #[test]
    fn deployment_decodes_program_binary_layout_with_padding() {
        let mut data = [0u8; 320];
        let mut offset = 0usize;
        data[offset..offset + 32].copy_from_slice(&[1u8; 32]);
        offset += 32;
        data[offset..offset + 32].copy_from_slice(&[2u8; 32]);
        offset += 32;
        data[offset..offset + 4].copy_from_slice(b"web1");
        offset += 64;
        data[offset..offset + 4].copy_from_slice(&3u32.to_le_bytes());
        offset += 4;
        data[offset..offset + 4].copy_from_slice(&4u32.to_le_bytes());
        offset += 4;
        data[offset..offset + 8].copy_from_slice(&8_000u64.to_le_bytes());
        offset += 8;
        data[offset..offset + 8].copy_from_slice(&16_000u64.to_le_bytes());
        offset += 8;
        data[offset..offset + 4].copy_from_slice(&100u32.to_le_bytes());
        offset += 4;
        data[offset..offset + 8].copy_from_slice(&1_000u64.to_le_bytes());
        offset += 8;
        data[offset..offset + 8].copy_from_slice(&10u64.to_le_bytes());
        offset += 8;
        data[offset..offset + 8].copy_from_slice(&20u64.to_le_bytes());
        offset += 8;
        data[offset] = DeploymentStatus::Running as u8;
        offset += 1;
        data[offset..offset + 8].copy_from_slice(&11i64.to_le_bytes());
        offset += 8;
        data[offset..offset + 8].copy_from_slice(&12i64.to_le_bytes());
        offset += 8;
        data[offset..offset + 8].copy_from_slice(&13i64.to_le_bytes());

        let deployment = Deployment::try_from_slice(&data).unwrap();
        assert_eq!(deployment.owner, Pubkey::new_from_array([1u8; 32]));
        assert_eq!(deployment.provider, Pubkey::new_from_array([2u8; 32]));
        assert_eq!(&deployment.name[..4], b"web1");
        assert_eq!(deployment.container_count, 3);
        assert_eq!(deployment.total_cpu_cores, 4);
        assert_eq!(deployment.total_memory_bytes, 8_000);
        assert_eq!(deployment.total_storage_bytes, 16_000);
        assert_eq!(deployment.total_network_mbps, 100);
        assert_eq!(deployment.deposit, 1_000);
        assert_eq!(deployment.burn_rate, 10);
        assert_eq!(deployment.spent, 20);
        assert_eq!(deployment.status, DeploymentStatus::Running);
        assert_eq!(deployment.created_at, 11);
        assert_eq!(deployment.started_at, 12);
        assert_eq!(deployment.paused_at, 13);
    }
}
