// Instruction codes
pub const INITIALIZE: u8 = 0;
pub const REGISTER: u8 = 1;
pub const PAUSE: u8 = 2;
pub const RESUME: u8 = 3;
pub const SLASH: u8 = 4;
pub const UPDATE_REPUTATION: u8 = 5;

use solana_program::program_error::ProgramError;
use ::core::convert::TryInto;
use ::core::fmt;

#[derive(Debug)]
pub enum Instruction {
    Initialize,
    Register { cpu_cores: u32, stake_amount: u64 },
    Pause,
    Resume,
    Slash,
    UpdateReputation { uptime_percent: u32 },
    Unknown,
}

pub fn unpack(data: &[u8]) -> Result<Instruction, ProgramError> {
    if data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    match data[0] {
        0 => Ok(Instruction::Initialize),
        1 => {
            if data.len() < 13 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let cpu_cores = u32::from_le_bytes(data[1..5].try_into().unwrap());
            let stake_amount = u64::from_le_bytes(data[5..13].try_into().unwrap());
            Ok(Instruction::Register { cpu_cores, stake_amount })
        }
        2 => Ok(Instruction::Pause),
        3 => Ok(Instruction::Resume),
        4 => Ok(Instruction::Slash),
        5 => {
            if data.len() < 5 {
                return Err(ProgramError::InvalidInstructionData);
            }
            let uptime_percent = u32::from_le_bytes(data[1..5].try_into().unwrap());
            Ok(Instruction::UpdateReputation { uptime_percent })
        }
        _ => Ok(Instruction::Unknown),
    }
}