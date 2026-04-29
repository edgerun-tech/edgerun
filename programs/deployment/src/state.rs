use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

pub const SIZE: usize = 448;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[borsh(use_discriminant = true)]
pub enum DeploymentStatus {
    Created = 0,
    Running = 1,
    Paused = 2,
    Stopped = 3,
    Disputed = 4,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
#[repr(C)]
pub struct Deployment {
    pub owner: Pubkey,
    pub provider: Pubkey,
    pub name: [u8; 64],
    pub container_count: u32,
    pub total_cpu_cores: u32,
    pub total_memory_bytes: u64,
    pub total_storage_bytes: u64,
    pub total_network_mbps: u32,
    pub deposit: u64,
    pub burn_rate: u64,
    pub spent: u64,
    pub status: u8,
    pub created_at: i64,
    pub started_at: i64,
    pub paused_at: i64,
    pub bump_seed: u8,
    pub last_report_at: i64,
    pub cpu_cores_used: u32,
    pub memory_bytes_used: u64,
    pub storage_bytes_used: u64,
    pub network_bytes_sent: u64,
    pub current_core_hour: u64,
    pub current_ram_gib_hour: u64,
    pub current_storage_gib_hour: u64,
    pub current_network_mbit_hour: u64,
    pub pending_core_hour: u64,
    pub pending_ram_gib_hour: u64,
    pub pending_storage_gib_hour: u64,
    pub pending_network_mbit_hour: u64,
    pub pending_price_effective_at: i64,
    pub auto_stop_on_price_increase: u8,
    pub governance_authority: Pubkey,
    pub provider_earned: u64,
    pub buyer_refunded: u64,
    pub dao_slashed: u64,
}

impl Deployment {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let mut reader = data;
        Self::deserialize(&mut reader).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn pack(&self, data: &mut [u8]) -> Result<(), ProgramError> {
        if data.len() < SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let mut cursor = std::io::Cursor::new(data);
        self.serialize(&mut cursor)
            .map_err(|_| ProgramError::InvalidAccountData)
    }
}

impl Default for Deployment {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            provider: Pubkey::default(),
            name: [0u8; 64],
            container_count: 0,
            total_cpu_cores: 0,
            total_memory_bytes: 0,
            total_storage_bytes: 0,
            total_network_mbps: 0,
            deposit: 0,
            burn_rate: 0,
            spent: 0,
            status: 0,
            created_at: 0,
            started_at: 0,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 0,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
            current_core_hour: 0,
            current_ram_gib_hour: 0,
            current_storage_gib_hour: 0,
            current_network_mbit_hour: 0,
            pending_core_hour: 0,
            pending_ram_gib_hour: 0,
            pending_storage_gib_hour: 0,
            pending_network_mbit_hour: 0,
            pending_price_effective_at: 0,
            auto_stop_on_price_increase: 1,
            governance_authority: Pubkey::default(),
            provider_earned: 0,
            buyer_refunded: 0,
            dao_slashed: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_accepts_padded_account_data() {
        let deployment = Deployment {
            owner: Pubkey::new_unique(),
            provider: Pubkey::new_unique(),
            name: [7u8; 64],
            container_count: 3,
            total_cpu_cores: 4,
            total_memory_bytes: 8_000,
            total_storage_bytes: 16_000,
            total_network_mbps: 100,
            deposit: 1_000,
            burn_rate: 10,
            spent: 20,
            status: DeploymentStatus::Running as u8,
            created_at: 11,
            started_at: 12,
            paused_at: 13,
            bump_seed: 1,
            last_report_at: 14,
            cpu_cores_used: 2,
            memory_bytes_used: 4_000,
            storage_bytes_used: 5_000,
            network_bytes_sent: 6_000,
            current_core_hour: 10,
            current_ram_gib_hour: 11,
            current_storage_gib_hour: 12,
            current_network_mbit_hour: 13,
            pending_core_hour: 20,
            pending_ram_gib_hour: 21,
            pending_storage_gib_hour: 22,
            pending_network_mbit_hour: 23,
            pending_price_effective_at: 15,
            auto_stop_on_price_increase: 1,
            governance_authority: Pubkey::new_unique(),
            provider_earned: 30,
            buyer_refunded: 40,
            dao_slashed: 50,
        };
        let mut data = [0u8; SIZE];

        deployment.pack(&mut data).unwrap();
        let unpacked = Deployment::unpack(&data).unwrap();

        assert_eq!(unpacked.owner, deployment.owner);
        assert_eq!(unpacked.provider, deployment.provider);
        assert_eq!(unpacked.name, deployment.name);
        assert_eq!(unpacked.container_count, deployment.container_count);
        assert_eq!(unpacked.status, deployment.status);
        assert_eq!(unpacked.spent, deployment.spent);
        assert_eq!(unpacked.network_bytes_sent, deployment.network_bytes_sent);
        assert_eq!(
            unpacked.pending_price_effective_at,
            deployment.pending_price_effective_at
        );
        assert_eq!(unpacked.auto_stop_on_price_increase, 1);
        assert_eq!(
            unpacked.governance_authority,
            deployment.governance_authority
        );
        assert_eq!(unpacked.provider_earned, deployment.provider_earned);
        assert_eq!(unpacked.buyer_refunded, deployment.buyer_refunded);
        assert_eq!(unpacked.dao_slashed, deployment.dao_slashed);
    }
}
