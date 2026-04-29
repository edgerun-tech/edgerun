use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum DeploymentInstruction {
    Initialize {
        name: [u8; 64],
        provider: [u8; 32],
        container_count: u32,
        total_cpu_cores: u32,
        total_memory_bytes: u64,
        total_storage_bytes: u64,
        total_network_mbps: u32,
        deposit: u64,
        burn_rate: u64,
    },
    Start,
    Pause,
    Resume,
    Stop,
    Dispute,
    Resolve {
        refund_to_buyer: u64,
        slash_to_dao: u64,
    },
    TickBurn,
    ReportMetrics {
        cpu_cores_used: u32,
        memory_bytes_used: u64,
        storage_bytes_used: u64,
        network_bytes_sent: u64,
        container_count: u32,
    },
}

impl DeploymentInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }
        match data[0] {
            0 => {
                let mut ops = data[1..].as_ref();
                Ok(DeploymentInstruction::Initialize {
                    name: <[u8; 64]>::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    provider: <[u8; 32]>::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    container_count: u32::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    total_cpu_cores: u32::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    total_memory_bytes: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    total_storage_bytes: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    total_network_mbps: u32::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    deposit: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    burn_rate: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                })
            }
            1 => Ok(DeploymentInstruction::Start),
            2 => Ok(DeploymentInstruction::Pause),
            3 => Ok(DeploymentInstruction::Resume),
            4 => Ok(DeploymentInstruction::Stop),
            5 => Ok(DeploymentInstruction::Dispute),
            6 => {
                let mut ops = data[1..].as_ref();
                Ok(DeploymentInstruction::Resolve {
                    refund_to_buyer: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    slash_to_dao: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                })
            }
            7 => Ok(DeploymentInstruction::TickBurn),
            8 => {
                let mut ops = data[1..].as_ref();
                Ok(DeploymentInstruction::ReportMetrics {
                    cpu_cores_used: u32::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    memory_bytes_used: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    storage_bytes_used: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    network_bytes_sent: u64::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                    container_count: u32::deserialize(&mut ops)
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                })
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
