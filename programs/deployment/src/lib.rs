//! Deployment Contract Program
//!
//! On-chain deployment management with per-second burn and deposit handling.

use solana_program::{
    account_info::{AccountInfo, next_account_info},
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
            start(accounts)
        }
        instruction::DeploymentInstruction::Pause => {
            msg!("DeploymentContract: Pause");
            pause(accounts)
        }
        instruction::DeploymentInstruction::Resume => {
            msg!("DeploymentContract: Resume");
            resume(accounts)
        }
        instruction::DeploymentInstruction::Stop => {
            msg!("DeploymentContract: Stop");
            stop(accounts)
        }
        instruction::DeploymentInstruction::Dispute => {
            msg!("DeploymentContract: Dispute");
            dispute(accounts)
        }
        instruction::DeploymentInstruction::Resolve { 
            refund_to_buyer, 
            slash_to_dao,
        } => {
            msg!("DeploymentContract: Resolve");
            resolve(accounts, refund_to_buyer, slash_to_dao)
        }
        instruction::DeploymentInstruction::TickBurn => {
            msg!("DeploymentContract: TickBurn");
            tick_burn(accounts)
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

fn start(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

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

fn pause(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

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

fn resume(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

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

fn stop(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

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

fn dispute(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;

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
    accounts: &[AccountInfo],
    _refund_to_buyer: u64,
    _slash_to_dao: u64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let dao_account = next_account_info(account_iter)?;

    if !dao_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    deployment.status = DeploymentStatus::Stopped as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn tick_burn(accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.status != DeploymentStatus::Running as u8 {
        return Ok(());
    }

    deployment.spent = deployment.spent.saturating_add(deployment.burn_rate);
    
    if deployment.spent >= deployment.deposit {
        deployment.status = DeploymentStatus::Paused as u8;
    }

    deployment.pack(&mut data)?;
    Ok(())
}

fn report_metrics(
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

    if !provider_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    let now = Clock::get()?.unix_timestamp;
    deployment.last_report_at = now;
    deployment.cpu_cores_used = cpu_cores_used;
    deployment.memory_bytes_used = memory_bytes_used;
    deployment.storage_bytes_used = storage_bytes_used;
    deployment.network_bytes_sent = network_bytes_sent;

    deployment.pack(&mut data)?;
    Ok(())
}