use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

pub const SIZE: usize = 320;

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
}

impl Deployment {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        Self::try_from_slice(data).map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn pack(&self, data: &mut [u8]) -> Result<(), ProgramError> {
        if data.len() < SIZE {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let mut cursor = std::io::Cursor::new(data);
        self.serialize(&mut cursor).map_err(|_| ProgramError::InvalidAccountData)
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
        }
    }
}