//! Provider Registry Program
//! 
//! On-chain registry for compute providers who offer resources (CPU, RAM, storage, bandwidth).
//! Providers stake collateral and register their available capacity.

use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

const PROVIDER_SIZE: usize = 128;

// Collateral constants (USDC micro-units)
const COLLATERAL_PER_CORE: u64 = 100_000_000; // 100 USDC

solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    match instruction_data[0] {
        0 => initialize(accounts),
        1 => register(accounts, &instruction_data[1..]),
        2 => pause(accounts),
        3 => resume(accounts),
        4 => slash(accounts),
        5 => update_reputation(accounts, &instruction_data[1..]),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn initialize(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let _provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Initialize provider data: [authority(32), collateral(8), cpu(4), memory(8), storage(8), network(4), earnings(8), slash(4), uptime(4), status(1), ...]
    let mut data = _provider.try_borrow_mut_data()?;
    data[0..32].copy_from_slice(authority.key.as_ref());
    data[80] = 0; // Active status
    
    Ok(())
}

fn register(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 12 {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let cpu_cores = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let stake_amount = u64::from_le_bytes(data[4..12].try_into().unwrap());

    let min_stake = COLLATERAL_PER_CORE * cpu_cores as u64;
    if stake_amount < min_stake {
        return Err(ProgramError::Custom(1)); // InsufficientCollateral
    }

    let mut account_data = provider.try_borrow_mut_data()?;
    account_data[0..32].copy_from_slice(authority.key.as_ref());
    account_data[32..40].copy_from_slice(&stake_amount.to_le_bytes());
    account_data[40..44].copy_from_slice(&cpu_cores.to_le_bytes());
    account_data[80] = 0; // Active

    Ok(())
}

fn pause(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = provider.try_borrow_mut_data()?;
    if data[0..32] != *authority.key.as_ref() {
        return Err(ProgramError::Custom(2)); // Unauthorized
    }

    data[80] = 1; // Paused
    Ok(())
}

fn resume(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = provider.try_borrow_mut_data()?;
    if data[0..32] != *authority.key.as_ref() {
        return Err(ProgramError::Custom(2)); // Unauthorized
    }

    data[80] = 0; // Active
    Ok(())
}

fn slash(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let _dao = next_account_info(account_iter)?;

    if !_dao.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = provider.try_borrow_mut_data()?;
    data[32..40].fill(0); // Slash collateral
    data[80] = 2; // Slashed
    let slash_count = u32::from_le_bytes(data[72..76].try_into().unwrap());
    data[72..76].copy_from_slice(&(slash_count + 1).to_le_bytes());

    Ok(())
}

fn update_reputation(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 4 {
        return Err(ProgramError::InvalidInstructionData);
    }
    
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let _reputation = next_account_info(account_iter)?;

    if !_reputation.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let uptime = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let mut account_data = provider.try_borrow_mut_data()?;
    account_data[76..80].copy_from_slice(&uptime.to_le_bytes());

    Ok(())
}

use ::core::convert::TryInto;