//! Provider Registry Program
//!
//! On-chain registry for compute providers who offer resources (CPU, RAM, storage, bandwidth).
//! Providers stake collateral and register their available capacity.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};

// Collateral constants (USDC micro-units)
const COLLATERAL_PER_CORE: u64 = 100_000_000; // 100 USDC
const COLLATERAL_PER_GIB_RAM: u64 = 50_000_000;
const COLLATERAL_PER_GIB_STORAGE: u64 = 10_000_000;
const COLLATERAL_PER_MBIT: u64 = 5_000_000;
const MAX_UPTIME_PERCENT_BPS: u32 = 10_000;
const PROVIDER_SIZE: usize = 128;

solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
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
        0 => initialize(program_id, accounts),
        1 => register(program_id, accounts, &instruction_data[1..]),
        2 => pause(program_id, accounts),
        3 => resume(program_id, accounts),
        4 => slash(program_id, accounts),
        5 => update_reputation(program_id, accounts, &instruction_data[1..]),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let _provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    require_program_owned(_provider, program_id)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Initialize provider data: [authority(32), collateral(8), cpu(4), memory(8), storage(8), network(4), earnings(8), slash(4), uptime(4), status(1), ...]
    let mut data = _provider.try_borrow_mut_data()?;
    require_uninitialized_provider(&data)?;
    data[0..32].copy_from_slice(authority.key.as_ref());
    data[80] = 0; // Active status

    Ok(())
}

fn register(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    require_program_owned(provider, program_id)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    {
        let account_data = provider.try_borrow_data()?;
        require_uninitialized_provider(&account_data)?;
    }

    let cpu_cores = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let stake_amount = u64::from_le_bytes(data[4..12].try_into().unwrap());
    let memory_bytes = u64::from_le_bytes(data[12..20].try_into().unwrap());
    let storage_bytes = u64::from_le_bytes(data[20..28].try_into().unwrap());
    let network_mbits = u32::from_le_bytes(data[28..32].try_into().unwrap());

    let ram_gib = memory_bytes.div_ceil(1024 * 1024 * 1024);
    let storage_gib = storage_bytes.div_ceil(1024 * 1024 * 1024);
    let min_stake = COLLATERAL_PER_CORE
        .saturating_mul(cpu_cores as u64)
        .saturating_add(COLLATERAL_PER_GIB_RAM.saturating_mul(ram_gib))
        .saturating_add(COLLATERAL_PER_GIB_STORAGE.saturating_mul(storage_gib))
        .saturating_add(COLLATERAL_PER_MBIT.saturating_mul(network_mbits as u64));
    if stake_amount < min_stake {
        return Err(ProgramError::Custom(1)); // InsufficientCollateral
    }

    transfer_from_signer(authority, provider, system_program, stake_amount)?;

    let mut account_data = provider.try_borrow_mut_data()?;
    account_data[0..32].copy_from_slice(authority.key.as_ref());
    account_data[32..40].copy_from_slice(&stake_amount.to_le_bytes());
    account_data[40..44].copy_from_slice(&cpu_cores.to_le_bytes());
    account_data[44..52].copy_from_slice(&memory_bytes.to_le_bytes());
    account_data[52..60].copy_from_slice(&storage_bytes.to_le_bytes());
    account_data[60..64].copy_from_slice(&network_mbits.to_le_bytes());
    account_data[80] = 0; // Active

    Ok(())
}

fn pause(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    require_program_owned(provider, program_id)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = provider.try_borrow_mut_data()?;
    if data[0..32] != *authority.key.as_ref() {
        return Err(ProgramError::Custom(2)); // Unauthorized
    }

    if data[80] == 2 {
        return Err(ProgramError::Custom(4)); // InvalidState
    }

    data[80] = 1; // Paused
    Ok(())
}

fn resume(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let authority = next_account_info(account_iter)?;

    require_program_owned(provider, program_id)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = provider.try_borrow_mut_data()?;
    if data[0..32] != *authority.key.as_ref() {
        return Err(ProgramError::Custom(2)); // Unauthorized
    }

    if data[80] != 1 {
        return Err(ProgramError::Custom(4)); // InvalidState
    }

    data[80] = 0; // Active
    Ok(())
}

fn slash(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let _dao = next_account_info(account_iter)?;

    require_program_owned(provider, program_id)?;

    if !_dao.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = provider.try_borrow_mut_data()?;
    if data[0..32] != *_dao.key.as_ref() {
        return Err(ProgramError::Custom(2)); // Unauthorized
    }

    if data[80] == 2 {
        return Err(ProgramError::Custom(4)); // InvalidState
    }

    let collateral = u64::from_le_bytes(data[32..40].try_into().unwrap());
    transfer_lamports(provider, _dao, collateral)?;
    data[32..40].fill(0); // Slash collateral
    data[80] = 2; // Slashed
    let slash_count = u32::from_le_bytes(data[72..76].try_into().unwrap());
    data[72..76].copy_from_slice(&(slash_count + 1).to_le_bytes());

    Ok(())
}

fn update_reputation(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 4 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let account_iter = &mut accounts.iter();
    let provider = next_account_info(account_iter)?;
    let _reputation = next_account_info(account_iter)?;

    require_program_owned(provider, program_id)?;

    if !_reputation.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut account_data = provider.try_borrow_mut_data()?;
    if account_data[0..32] != *_reputation.key.as_ref() {
        return Err(ProgramError::Custom(2)); // Unauthorized
    }

    if account_data[80] == 2 {
        return Err(ProgramError::Custom(4)); // InvalidState
    }

    let uptime_percent_bps = u32::from_le_bytes(data[0..4].try_into().unwrap());
    if uptime_percent_bps > MAX_UPTIME_PERCENT_BPS {
        return Err(ProgramError::InvalidArgument);
    }
    account_data[76..80].copy_from_slice(&uptime_percent_bps.to_le_bytes());

    Ok(())
}

fn require_program_owned(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
    if account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

fn require_uninitialized_provider(data: &[u8]) -> ProgramResult {
    if data.len() < PROVIDER_SIZE {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let has_authority = data[0..32].iter().any(|byte| *byte != 0);
    let collateral = u64::from_le_bytes(data[32..40].try_into().unwrap());
    let cpu_cores = u32::from_le_bytes(data[40..44].try_into().unwrap());
    let status = data[80];
    if has_authority || collateral != 0 || cpu_cores != 0 || status != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}

fn transfer_lamports(from: &AccountInfo, to: &AccountInfo, amount: u64) -> ProgramResult {
    if amount == 0 {
        return Ok(());
    }

    {
        let mut from_lamports = from.try_borrow_mut_lamports()?;
        if **from_lamports < amount {
            return Err(ProgramError::InsufficientFunds);
        }
        **from_lamports -= amount;
    }

    let mut to_lamports = to.try_borrow_mut_lamports()?;
    **to_lamports = to_lamports
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(())
}

fn transfer_from_signer<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Ok(());
    }
    if !from.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if *system_program_account.key != system_program_id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    invoke(
        &system_transfer_instruction(from.key, to.key, amount),
        &[from.clone(), to.clone(), system_program_account.clone()],
    )
}

fn system_program_id() -> Pubkey {
    Pubkey::default()
}

fn system_transfer_instruction(from: &Pubkey, to: &Pubkey, amount: u64) -> Instruction {
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: system_program_id(),
        accounts: vec![AccountMeta::new(*from, true), AccountMeta::new(*to, false)],
        data,
    }
}

use ::core::convert::TryInto;

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::account_info::AccountInfo;

    fn account<'a>(
        key: &'a Pubkey,
        owner: &'a Pubkey,
        is_signer: bool,
        data: &'a mut [u8],
        lamports: &'a mut u64,
    ) -> AccountInfo<'a> {
        AccountInfo::new(key, is_signer, true, lamports, data, owner, false)
    }

    fn register_data(
        cpu_cores: u32,
        stake_amount: u64,
        memory_bytes: u64,
        storage_bytes: u64,
        network_mbits: u32,
    ) -> Vec<u8> {
        let mut data = vec![1];
        data.extend_from_slice(&cpu_cores.to_le_bytes());
        data.extend_from_slice(&stake_amount.to_le_bytes());
        data.extend_from_slice(&memory_bytes.to_le_bytes());
        data.extend_from_slice(&storage_bytes.to_le_bytes());
        data.extend_from_slice(&network_mbits.to_le_bytes());
        data
    }

    #[test]
    fn register_persists_full_capacity_layout() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let mut provider_lamports = 0;
        let mut authority_lamports = 2_000_000_000;
        let mut system_lamports = 0;
        let mut provider_data = [0u8; 128];
        let mut authority_data = [];
        let mut system_data = [];
        let system_key = system_program_id();

        let provider = account(
            &provider_key,
            &program_id,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let authority = account(
            &authority_key,
            &program_id,
            true,
            &mut authority_data,
            &mut authority_lamports,
        );
        let system = account(
            &system_key,
            &system_key,
            false,
            &mut system_data,
            &mut system_lamports,
        );
        let accounts = vec![provider, authority, system];

        let memory_bytes = 8 * 1024 * 1024 * 1024;
        let storage_bytes = 16 * 1024 * 1024 * 1024;
        let network_mbits = 100;
        let stake = 4 * COLLATERAL_PER_CORE
            + 8 * COLLATERAL_PER_GIB_RAM
            + 16 * COLLATERAL_PER_GIB_STORAGE
            + 100 * COLLATERAL_PER_MBIT;
        let data = register_data(4, stake, memory_bytes, storage_bytes, network_mbits);

        process_instruction(&program_id, &accounts, &data).unwrap();

        assert_eq!(&provider_data[0..32], authority_key.as_ref());
        assert_eq!(
            u64::from_le_bytes(provider_data[32..40].try_into().unwrap()),
            stake
        );
        assert_eq!(
            u32::from_le_bytes(provider_data[40..44].try_into().unwrap()),
            4
        );
        assert_eq!(
            u64::from_le_bytes(provider_data[44..52].try_into().unwrap()),
            memory_bytes
        );
        assert_eq!(
            u64::from_le_bytes(provider_data[52..60].try_into().unwrap()),
            storage_bytes
        );
        assert_eq!(
            u32::from_le_bytes(provider_data[60..64].try_into().unwrap()),
            network_mbits
        );
        assert_eq!(provider_data[80], 0);
    }

    #[test]
    fn register_rejects_insufficient_collateral() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let mut provider_lamports = 0;
        let mut authority_lamports = 0;
        let mut system_lamports = 0;
        let mut provider_data = [0u8; 128];
        let mut authority_data = [];
        let mut system_data = [];
        let system_key = system_program_id();

        let provider = account(
            &provider_key,
            &program_id,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let authority = account(
            &authority_key,
            &program_id,
            true,
            &mut authority_data,
            &mut authority_lamports,
        );
        let system = account(
            &system_key,
            &system_key,
            false,
            &mut system_data,
            &mut system_lamports,
        );
        let accounts = vec![provider, authority, system];
        let data = register_data(4, 1, 8 * 1024 * 1024 * 1024, 0, 0);

        assert_eq!(
            process_instruction(&program_id, &accounts, &data),
            Err(ProgramError::Custom(1))
        );
    }

    #[test]
    fn pause_and_resume_require_authority() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let wrong_key = Pubkey::new_unique();
        let mut provider_lamports = 0;
        let mut authority_lamports = 0;
        let mut wrong_lamports = 0;
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(authority_key.as_ref());
        let mut authority_data = [];
        let mut wrong_data = [];

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let authority = account(
                &authority_key,
                &program_id,
                true,
                &mut authority_data,
                &mut authority_lamports,
            );
            let accounts = vec![provider, authority];
            process_instruction(&program_id, &accounts, &[2]).unwrap();
        }
        assert_eq!(provider_data[80], 1);

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let wrong_authority = account(
                &wrong_key,
                &program_id,
                true,
                &mut wrong_data,
                &mut wrong_lamports,
            );
            let accounts = vec![provider, wrong_authority];
            assert_eq!(
                process_instruction(&program_id, &accounts, &[3]),
                Err(ProgramError::Custom(2))
            );
        }
        assert_eq!(provider_data[80], 1);

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let authority = account(
                &authority_key,
                &program_id,
                true,
                &mut authority_data,
                &mut authority_lamports,
            );
            let accounts = vec![provider, authority];
            process_instruction(&program_id, &accounts, &[3]).unwrap();
        }
        assert_eq!(provider_data[80], 0);
    }

    #[test]
    fn slash_and_reputation_require_authority() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let wrong_key = Pubkey::new_unique();
        let mut provider_lamports = 0;
        let mut authority_lamports = 0;
        let mut wrong_lamports = 0;
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(authority_key.as_ref());
        provider_data[32..40].copy_from_slice(&100u64.to_le_bytes());
        let mut authority_data = [];
        let mut wrong_data = [];

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let wrong = account(
                &wrong_key,
                &program_id,
                true,
                &mut wrong_data,
                &mut wrong_lamports,
            );
            let accounts = vec![provider, wrong];
            assert_eq!(
                process_instruction(&program_id, &accounts, &[4]),
                Err(ProgramError::Custom(2))
            );
        }
        assert_eq!(
            u64::from_le_bytes(provider_data[32..40].try_into().unwrap()),
            100
        );

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let wrong = account(
                &wrong_key,
                &program_id,
                true,
                &mut wrong_data,
                &mut wrong_lamports,
            );
            let accounts = vec![provider, wrong];
            let mut data = vec![5];
            data.extend_from_slice(&9_999u32.to_le_bytes());
            assert_eq!(
                process_instruction(&program_id, &accounts, &data),
                Err(ProgramError::Custom(2))
            );
        }
        assert_eq!(
            u32::from_le_bytes(provider_data[76..80].try_into().unwrap()),
            0
        );

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let authority = account(
                &authority_key,
                &program_id,
                true,
                &mut authority_data,
                &mut authority_lamports,
            );
            let accounts = vec![provider, authority];
            let mut data = vec![5];
            data.extend_from_slice(&9_999u32.to_le_bytes());
            process_instruction(&program_id, &accounts, &data).unwrap();
        }
        assert_eq!(
            u32::from_le_bytes(provider_data[76..80].try_into().unwrap()),
            9_999
        );
    }

    #[test]
    fn slash_transfers_collateral_to_authority() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let mut provider_lamports = 500;
        let mut authority_lamports = 10;
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(authority_key.as_ref());
        provider_data[32..40].copy_from_slice(&500u64.to_le_bytes());
        let mut authority_data = [];

        let provider = account(
            &provider_key,
            &program_id,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let authority = account(
            &authority_key,
            &program_id,
            true,
            &mut authority_data,
            &mut authority_lamports,
        );
        let accounts = vec![provider, authority];

        process_instruction(&program_id, &accounts, &[4]).unwrap();

        assert_eq!(provider_lamports, 0);
        assert_eq!(authority_lamports, 510);
        assert_eq!(
            u64::from_le_bytes(provider_data[32..40].try_into().unwrap()),
            0
        );
        assert_eq!(provider_data[80], 2);
    }

    #[test]
    fn slashed_provider_rejects_resume_reslash_and_reputation() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let mut provider_lamports = 0;
        let mut authority_lamports = 0;
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(authority_key.as_ref());
        provider_data[80] = 2;
        let mut authority_data = [];

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let authority = account(
                &authority_key,
                &program_id,
                true,
                &mut authority_data,
                &mut authority_lamports,
            );
            let accounts = vec![provider, authority];
            assert_eq!(
                process_instruction(&program_id, &accounts, &[3]),
                Err(ProgramError::Custom(4))
            );
        }

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let authority = account(
                &authority_key,
                &program_id,
                true,
                &mut authority_data,
                &mut authority_lamports,
            );
            let accounts = vec![provider, authority];
            assert_eq!(
                process_instruction(&program_id, &accounts, &[4]),
                Err(ProgramError::Custom(4))
            );
        }

        {
            let provider = account(
                &provider_key,
                &program_id,
                false,
                &mut provider_data,
                &mut provider_lamports,
            );
            let authority = account(
                &authority_key,
                &program_id,
                true,
                &mut authority_data,
                &mut authority_lamports,
            );
            let accounts = vec![provider, authority];
            let mut data = vec![5];
            data.extend_from_slice(&9_999u32.to_le_bytes());
            assert_eq!(
                process_instruction(&program_id, &accounts, &data),
                Err(ProgramError::Custom(4))
            );
        }

        assert_eq!(provider_data[80], 2);
        assert_eq!(
            u32::from_le_bytes(provider_data[76..80].try_into().unwrap()),
            0
        );
    }

    #[test]
    fn reputation_rejects_uptime_above_one_hundred_percent() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let mut provider_lamports = 0;
        let mut authority_lamports = 0;
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(authority_key.as_ref());
        let mut authority_data = [];

        let provider = account(
            &provider_key,
            &program_id,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let authority = account(
            &authority_key,
            &program_id,
            true,
            &mut authority_data,
            &mut authority_lamports,
        );
        let accounts = vec![provider, authority];
        let mut data = vec![5];
        data.extend_from_slice(&10_001u32.to_le_bytes());

        assert_eq!(
            process_instruction(&program_id, &accounts, &data),
            Err(ProgramError::InvalidArgument)
        );
        assert_eq!(
            u32::from_le_bytes(provider_data[76..80].try_into().unwrap()),
            0
        );
    }

    #[test]
    fn rejects_provider_account_not_owned_by_program() {
        let program_id = Pubkey::new_unique();
        let wrong_owner = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let mut provider_lamports = 0;
        let mut authority_lamports = 0;
        let mut system_lamports = 0;
        let mut provider_data = [0u8; 128];
        let mut authority_data = [];
        let mut system_data = [];
        let system_key = system_program_id();
        let stake = 2 * COLLATERAL_PER_CORE
            + 4 * COLLATERAL_PER_GIB_RAM
            + 8 * COLLATERAL_PER_GIB_STORAGE
            + 100 * COLLATERAL_PER_MBIT;
        let data = register_data(
            2,
            stake,
            4 * 1024 * 1024 * 1024,
            8 * 1024 * 1024 * 1024,
            100,
        );

        let provider_account = account(
            &provider_key,
            &wrong_owner,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let authority_account = account(
            &authority_key,
            &program_id,
            true,
            &mut authority_data,
            &mut authority_lamports,
        );
        let system_account = account(
            &system_key,
            &system_key,
            false,
            &mut system_data,
            &mut system_lamports,
        );
        let accounts = vec![provider_account, authority_account, system_account];

        assert_eq!(
            process_instruction(&program_id, &accounts, &data),
            Err(ProgramError::IncorrectProgramId)
        );
    }

    #[test]
    fn register_rejects_existing_provider_state_before_funding() {
        let program_id = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let authority_key = Pubkey::new_unique();
        let system_key = system_program_id();
        let mut provider_lamports = 0;
        let mut authority_lamports = 2_000_000_000;
        let mut system_lamports = 0;
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(authority_key.as_ref());
        provider_data[32..40].copy_from_slice(&100u64.to_le_bytes());
        let mut authority_data = [];
        let mut system_data = [];
        let stake = 2 * COLLATERAL_PER_CORE
            + 4 * COLLATERAL_PER_GIB_RAM
            + 8 * COLLATERAL_PER_GIB_STORAGE
            + 100 * COLLATERAL_PER_MBIT;
        let data = register_data(
            2,
            stake,
            4 * 1024 * 1024 * 1024,
            8 * 1024 * 1024 * 1024,
            100,
        );

        let provider = account(
            &provider_key,
            &program_id,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let authority = account(
            &authority_key,
            &program_id,
            true,
            &mut authority_data,
            &mut authority_lamports,
        );
        let system = account(
            &system_key,
            &system_key,
            false,
            &mut system_data,
            &mut system_lamports,
        );
        let accounts = vec![provider, authority, system];

        assert_eq!(
            process_instruction(&program_id, &accounts, &data),
            Err(ProgramError::AccountAlreadyInitialized)
        );
        assert_eq!(provider_lamports, 0);
        assert_eq!(authority_lamports, 2_000_000_000);
    }
}
