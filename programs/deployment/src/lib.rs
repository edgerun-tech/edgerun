//! Deployment Contract Program
//!
//! On-chain deployment management with per-second burn and deposit handling.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

pub mod error;
pub mod instruction;
pub mod state;

use error::DeploymentError;
use state::Deployment;
use state::DeploymentStatus;

solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let instruction = instruction::DeploymentInstruction::unpack(instruction_data)?;

    match instruction {
        instruction::DeploymentInstruction::Initialize {
            name,
            provider,
            container_count,
            total_cpu_cores,
            total_memory_bytes,
            total_storage_bytes,
            total_network_mbps,
            deposit,
            burn_rate,
        } => {
            msg!("DeploymentContract: Initialize");
            initialize(
                program_id,
                accounts,
                name,
                provider,
                container_count,
                total_cpu_cores,
                total_memory_bytes,
                total_storage_bytes,
                total_network_mbps,
                deposit,
                burn_rate,
            )
        }
        instruction::DeploymentInstruction::Start => {
            msg!("DeploymentContract: Start");
            start(program_id, accounts)
        }
        instruction::DeploymentInstruction::Pause => {
            msg!("DeploymentContract: Pause");
            pause(program_id, accounts)
        }
        instruction::DeploymentInstruction::Resume => {
            msg!("DeploymentContract: Resume");
            resume(program_id, accounts)
        }
        instruction::DeploymentInstruction::Stop => {
            msg!("DeploymentContract: Stop");
            stop(program_id, accounts)
        }
        instruction::DeploymentInstruction::Dispute => {
            msg!("DeploymentContract: Dispute");
            dispute(program_id, accounts)
        }
        instruction::DeploymentInstruction::Resolve {
            refund_to_buyer,
            slash_to_dao,
        } => {
            msg!("DeploymentContract: Resolve");
            resolve(program_id, accounts, refund_to_buyer, slash_to_dao)
        }
        instruction::DeploymentInstruction::TickBurn => {
            msg!("DeploymentContract: TickBurn");
            tick_burn(program_id, accounts)
        }
        instruction::DeploymentInstruction::ReportMetrics {
            cpu_cores_used,
            memory_bytes_used,
            storage_bytes_used,
            network_bytes_sent,
            container_count,
        } => {
            msg!("DeploymentContract: ReportMetrics");
            report_metrics(
                program_id,
                accounts,
                cpu_cores_used,
                memory_bytes_used,
                storage_bytes_used,
                network_bytes_sent,
                container_count,
            )
        }
    }
}

fn initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: [u8; 64],
    provider: [u8; 32],
    container_count: u32,
    total_cpu_cores: u32,
    total_memory_bytes: u64,
    total_storage_bytes: u64,
    total_network_mbps: u32,
    deposit: u64,
    burn_rate: u64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if deposit == 0 {
        return Err(DeploymentError::ZeroDeposit.into());
    }

    let now = Clock::get()?.unix_timestamp;

    let deployment = Deployment {
        owner: *owner_account.key,
        provider: provider.into(),
        name,
        container_count,
        total_cpu_cores,
        total_memory_bytes,
        total_storage_bytes,
        total_network_mbps,
        deposit,
        burn_rate,
        spent: 0,
        status: DeploymentStatus::Created as u8,
        created_at: now,
        started_at: 0,
        paused_at: 0,
        bump_seed: 0,
        last_report_at: 0,
        cpu_cores_used: 0,
        memory_bytes_used: 0,
        storage_bytes_used: 0,
        network_bytes_sent: 0,
    };

    let mut data = deployment_account.try_borrow_mut_data()?;
    deployment.pack(&mut data)?;
    Ok(())
}

fn start(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.owner != *owner_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    if deployment.status != DeploymentStatus::Created as u8
        && deployment.status != DeploymentStatus::Paused as u8
    {
        return Err(DeploymentError::InvalidState.into());
    }

    let now = Clock::get()?.unix_timestamp;
    deployment.started_at = now;
    deployment.status = DeploymentStatus::Running as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn pause(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.owner != *owner_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    if deployment.status != DeploymentStatus::Running as u8 {
        return Err(DeploymentError::InvalidState.into());
    }

    let now = Clock::get()?.unix_timestamp;
    deployment.paused_at = now;
    deployment.status = DeploymentStatus::Paused as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn resume(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.owner != *owner_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    if deployment.status != DeploymentStatus::Paused as u8 {
        return Err(DeploymentError::InvalidState.into());
    }

    let now = Clock::get()?.unix_timestamp;
    deployment.started_at = now;
    deployment.status = DeploymentStatus::Running as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn stop(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.owner != *owner_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    deployment.status = DeploymentStatus::Stopped as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn dispute(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.owner != *owner_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    deployment.status = DeploymentStatus::Disputed as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn resolve(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _refund_to_buyer: u64,
    _slash_to_dao: u64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let dao_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !dao_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.owner != *dao_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    deployment.status = DeploymentStatus::Stopped as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn tick_burn(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    let now = Clock::get()?.unix_timestamp;
    apply_elapsed_burn(&mut deployment, now);

    deployment.pack(&mut data)?;
    Ok(())
}

fn apply_elapsed_burn(deployment: &mut Deployment, now: i64) {
    if deployment.status != DeploymentStatus::Running as u8 {
        return;
    }

    if deployment.started_at <= 0 || now <= deployment.started_at {
        return;
    }

    let elapsed_seconds = (now - deployment.started_at) as u64;
    let burned = deployment.burn_rate.saturating_mul(elapsed_seconds);
    deployment.spent = deployment
        .spent
        .saturating_add(burned)
        .min(deployment.deposit);
    deployment.started_at = now;

    if deployment.spent >= deployment.deposit {
        deployment.status = DeploymentStatus::Paused as u8;
        deployment.paused_at = now;
    }
}

fn report_metrics(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    cpu_cores_used: u32,
    memory_bytes_used: u64,
    storage_bytes_used: u64,
    network_bytes_sent: u64,
    _container_count: u32,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let provider_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !provider_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.provider != *provider_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    let now = Clock::get()?.unix_timestamp;
    deployment.last_report_at = now;
    deployment.cpu_cores_used = cpu_cores_used;
    deployment.memory_bytes_used = memory_bytes_used;
    deployment.storage_bytes_used = storage_bytes_used;
    deployment.network_bytes_sent = network_bytes_sent;

    deployment.pack(&mut data)?;
    Ok(())
}

fn require_program_owned(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
    if account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

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

    #[test]
    fn report_metrics_rejects_non_assigned_provider() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let assigned_provider = Pubkey::new_unique();
        let wrong_provider = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut deployment_lamports = 0;
        let mut provider_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut provider_data = [];

        state::Deployment {
            owner,
            provider: assigned_provider,
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1000,
            burn_rate: 1,
            spent: 0,
            status: state::DeploymentStatus::Running as u8,
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
        .pack(&mut deployment_data)
        .unwrap();

        let deployment_account = account(
            &deployment_key,
            &program_id,
            false,
            &mut deployment_data,
            &mut deployment_lamports,
        );
        let provider_account = account(
            &wrong_provider,
            &program_id,
            true,
            &mut provider_data,
            &mut provider_lamports,
        );
        let accounts = vec![deployment_account, provider_account];
        let mut instruction = vec![8];
        instruction.extend_from_slice(&1u32.to_le_bytes());
        instruction.extend_from_slice(&2u64.to_le_bytes());
        instruction.extend_from_slice(&3u64.to_le_bytes());
        instruction.extend_from_slice(&4u64.to_le_bytes());
        instruction.extend_from_slice(&1u32.to_le_bytes());

        assert_eq!(
            process_instruction(&program_id, &accounts, &instruction),
            Err(DeploymentError::Unauthorized.into())
        );
    }

    #[test]
    fn resolve_requires_owner_until_governance_is_modeled() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let wrong_resolver = Pubkey::new_unique();
        let mut deployment_lamports = 0;
        let mut resolver_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut resolver_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1000,
            burn_rate: 1,
            spent: 0,
            status: state::DeploymentStatus::Disputed as u8,
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
        .pack(&mut deployment_data)
        .unwrap();

        let deployment_account = account(
            &deployment_key,
            &program_id,
            false,
            &mut deployment_data,
            &mut deployment_lamports,
        );
        let resolver_account = account(
            &wrong_resolver,
            &program_id,
            true,
            &mut resolver_data,
            &mut resolver_lamports,
        );
        let accounts = vec![deployment_account, resolver_account];
        let mut instruction = vec![6];
        instruction.extend_from_slice(&1u64.to_le_bytes());
        instruction.extend_from_slice(&1u64.to_le_bytes());

        assert_eq!(
            process_instruction(&program_id, &accounts, &instruction),
            Err(DeploymentError::Unauthorized.into())
        );
    }

    #[test]
    fn elapsed_burn_uses_seconds_since_last_tick() {
        let mut deployment = state::Deployment {
            owner: Pubkey::new_unique(),
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1000,
            burn_rate: 7,
            spent: 10,
            status: state::DeploymentStatus::Running as u8,
            created_at: 0,
            started_at: 100,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 0,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
        };

        apply_elapsed_burn(&mut deployment, 105);

        assert_eq!(deployment.spent, 45);
        assert_eq!(deployment.started_at, 105);
        assert_eq!(deployment.status, state::DeploymentStatus::Running as u8);
    }

    #[test]
    fn elapsed_burn_caps_at_deposit_and_pauses() {
        let mut deployment = state::Deployment {
            owner: Pubkey::new_unique(),
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 100,
            burn_rate: 20,
            spent: 70,
            status: state::DeploymentStatus::Running as u8,
            created_at: 0,
            started_at: 10,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 0,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
        };

        apply_elapsed_burn(&mut deployment, 12);

        assert_eq!(deployment.spent, 100);
        assert_eq!(deployment.started_at, 12);
        assert_eq!(deployment.paused_at, 12);
        assert_eq!(deployment.status, state::DeploymentStatus::Paused as u8);
    }

    #[test]
    fn rejects_deployment_account_not_owned_by_program() {
        let program_id = Pubkey::new_unique();
        let wrong_owner = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut deployment_lamports = 0;
        let mut owner_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut owner_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1000,
            burn_rate: 1,
            spent: 0,
            status: state::DeploymentStatus::Created as u8,
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
        .pack(&mut deployment_data)
        .unwrap();

        let deployment_account = account(
            &deployment_key,
            &wrong_owner,
            false,
            &mut deployment_data,
            &mut deployment_lamports,
        );
        let owner_account = account(
            &owner,
            &program_id,
            true,
            &mut owner_data,
            &mut owner_lamports,
        );
        let accounts = vec![deployment_account, owner_account];

        assert_eq!(
            process_instruction(&program_id, &accounts, &[1]),
            Err(ProgramError::IncorrectProgramId)
        );
    }
}
