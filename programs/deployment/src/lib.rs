//! Deployment Contract Program
//!
//! On-chain deployment management with per-second burn and deposit handling.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::{invoke, invoke_signed},
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

const CORE_HOUR: u64 = 10_000;
const RAM_GIB_HOUR: u64 = 5_000;
const STORAGE_GIB_HOUR: u64 = 1_000;
const NETWORK_MBIT_HOUR: u64 = 2_000;
const SECONDS_PER_HOUR: u64 = 3_600;
const PRICE_CHANGE_GRACE_SECONDS: i64 = 86_400;
const GIB: u64 = 1024 * 1024 * 1024;
const ESCROW_AUTHORITY_SEED: &[u8] = b"deployment-escrow";
const TOKEN_TRANSFER_INSTRUCTION: u8 = 3;
const TOKEN_ACCOUNT_LEN: usize = 165;
const TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
const TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;

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
            auto_stop_on_price_increase,
            governance_authority,
            payment_mint,
            escrow_token_account,
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
                auto_stop_on_price_increase,
                governance_authority,
                payment_mint,
                escrow_token_account,
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
            provider_payout,
            slash_to_dao,
        } => {
            msg!("DeploymentContract: Resolve");
            resolve(
                program_id,
                accounts,
                refund_to_buyer,
                provider_payout,
                slash_to_dao,
            )
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
        instruction::DeploymentInstruction::SchedulePricing {
            core_hour,
            ram_gib_hour,
            storage_gib_hour,
            network_mbit_hour,
            effective_at,
        } => {
            msg!("DeploymentContract: SchedulePricing");
            schedule_pricing(
                program_id,
                accounts,
                core_hour,
                ram_gib_hour,
                storage_gib_hour,
                network_mbit_hour,
                effective_at,
            )
        }
        instruction::DeploymentInstruction::AssignProvider { provider } => {
            msg!("DeploymentContract: AssignProvider");
            assign_provider(program_id, accounts, provider)
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
    auto_stop_on_price_increase: bool,
    governance_authority: [u8; 32],
    payment_mint: [u8; 32],
    escrow_token_account: [u8; 32],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let owner_account = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if deposit == 0 {
        return Err(DeploymentError::ZeroDeposit.into());
    }

    {
        let data = deployment_account.try_borrow_data()?;
        require_uninitialized_deployment(&data)?;
    }

    if Pubkey::new_from_array(provider) != Pubkey::default() {
        return Err(DeploymentError::Unauthorized.into());
    }

    let payment_mint = Pubkey::new_from_array(payment_mint);
    let escrow_token_account = Pubkey::new_from_array(escrow_token_account);
    let token_escrow_enabled =
        payment_mint != Pubkey::default() || escrow_token_account != Pubkey::default();
    if token_escrow_enabled {
        if payment_mint == Pubkey::default() || escrow_token_account == Pubkey::default() {
            return Err(ProgramError::InvalidArgument);
        }
        let owner_token_account = next_account_info(account_iter)?;
        let escrow_token_info = next_account_info(account_iter)?;
        let token_program = next_account_info(account_iter)?;
        if *escrow_token_info.key != escrow_token_account {
            return Err(ProgramError::InvalidArgument);
        }
        let (escrow_authority, _) = escrow_authority(program_id, deployment_account.key);
        validate_token_account(
            owner_token_account,
            token_program.key,
            &payment_mint,
            owner_account.key,
        )?;
        validate_token_account(
            escrow_token_info,
            token_program.key,
            &payment_mint,
            &escrow_authority,
        )?;
        transfer_spl_tokens(
            owner_token_account,
            escrow_token_info,
            owner_account,
            token_program,
            deposit,
        )?;
    } else {
        transfer_from_signer(owner_account, deployment_account, system_program, deposit)?;
    }

    let now = Clock::get()?.unix_timestamp;
    let governance_authority = Pubkey::new_from_array(governance_authority);
    let governance_authority = if governance_authority == Pubkey::default() {
        *owner_account.key
    } else {
        governance_authority
    };

    let deployment = Deployment {
        owner: *owner_account.key,
        provider: Pubkey::default(),
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
        current_core_hour: CORE_HOUR,
        current_ram_gib_hour: RAM_GIB_HOUR,
        current_storage_gib_hour: STORAGE_GIB_HOUR,
        current_network_mbit_hour: NETWORK_MBIT_HOUR,
        pending_core_hour: 0,
        pending_ram_gib_hour: 0,
        pending_storage_gib_hour: 0,
        pending_network_mbit_hour: 0,
        pending_price_effective_at: 0,
        auto_stop_on_price_increase: u8::from(auto_stop_on_price_increase),
        governance_authority,
        provider_earned: 0,
        buyer_refunded: 0,
        dao_slashed: 0,
        payment_mint,
        escrow_token_account,
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

    if deployment.provider == Pubkey::default() {
        return Err(DeploymentError::ProviderNotAssigned.into());
    }

    let now = Clock::get()?.unix_timestamp;
    apply_pending_pricing(&mut deployment, now);
    if deployment.status == DeploymentStatus::Stopped as u8 {
        deployment.pack(&mut data)?;
        return Ok(());
    }
    deployment.started_at = now;
    deployment.last_report_at = now;
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
    let provider_payout_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.owner != *owner_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    if deployment.status == DeploymentStatus::Disputed as u8 {
        return Err(DeploymentError::InvalidState.into());
    }

    apply_elapsed_burn_if_running(&mut deployment)?;
    let provider_payout = deployment.spent;
    let refund = deployment.deposit.saturating_sub(deployment.spent);
    if uses_token_escrow(&deployment) {
        let escrow_token_account = next_account_info(account_iter)?;
        let escrow_authority_account = next_account_info(account_iter)?;
        let buyer_refund_account = next_account_info(account_iter)?;
        let token_program = next_account_info(account_iter)?;
        transfer_from_escrow(
            program_id,
            deployment_account,
            &deployment,
            escrow_token_account,
            escrow_authority_account,
            provider_payout_account,
            token_program,
            provider_payout,
        )?;
        transfer_from_escrow(
            program_id,
            deployment_account,
            &deployment,
            escrow_token_account,
            escrow_authority_account,
            buyer_refund_account,
            token_program,
            refund,
        )?;
    } else {
        if provider_payout > 0 {
            transfer_lamports(deployment_account, provider_payout_account, provider_payout)?;
        }
        if refund > 0 {
            transfer_lamports(deployment_account, owner_account, refund)?;
        }
    }

    deployment.status = DeploymentStatus::Stopped as u8;
    deployment.provider_earned = provider_payout;
    deployment.buyer_refunded = refund;
    deployment.dao_slashed = 0;
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

    if deployment.status != DeploymentStatus::Running as u8
        && deployment.status != DeploymentStatus::Paused as u8
    {
        return Err(DeploymentError::InvalidState.into());
    }

    deployment.status = DeploymentStatus::Disputed as u8;
    deployment.pack(&mut data)?;
    Ok(())
}

fn resolve(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    refund_to_buyer: u64,
    provider_payout: u64,
    slash_to_dao: u64,
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

    if governance_authority(&deployment) != *dao_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    if deployment.status != DeploymentStatus::Disputed as u8 {
        return Err(DeploymentError::InvalidState.into());
    }

    apply_elapsed_burn_if_running(&mut deployment)?;
    let settlement_total = refund_to_buyer
        .saturating_add(provider_payout)
        .saturating_add(slash_to_dao);
    if settlement_total != deployment.deposit {
        return Err(ProgramError::InsufficientFunds);
    }

    if settlement_total > 0 {
        let refund_account = next_account_info(account_iter)?;
        let provider_payout_account = next_account_info(account_iter)?;
        let slash_account = next_account_info(account_iter)?;
        if uses_token_escrow(&deployment) {
            let escrow_token_account = next_account_info(account_iter)?;
            let escrow_authority_account = next_account_info(account_iter)?;
            let token_program = next_account_info(account_iter)?;
            transfer_from_escrow(
                program_id,
                deployment_account,
                &deployment,
                escrow_token_account,
                escrow_authority_account,
                refund_account,
                token_program,
                refund_to_buyer,
            )?;
            transfer_from_escrow(
                program_id,
                deployment_account,
                &deployment,
                escrow_token_account,
                escrow_authority_account,
                provider_payout_account,
                token_program,
                provider_payout,
            )?;
            transfer_from_escrow(
                program_id,
                deployment_account,
                &deployment,
                escrow_token_account,
                escrow_authority_account,
                slash_account,
                token_program,
                slash_to_dao,
            )?;
        } else {
            transfer_lamports(deployment_account, refund_account, refund_to_buyer)?;
            transfer_lamports(deployment_account, provider_payout_account, provider_payout)?;
            transfer_lamports(deployment_account, slash_account, slash_to_dao)?;
        }
    }

    deployment.status = DeploymentStatus::Stopped as u8;
    deployment.spent = provider_payout.saturating_add(slash_to_dao);
    deployment.provider_earned = provider_payout;
    deployment.buyer_refunded = refund_to_buyer;
    deployment.dao_slashed = slash_to_dao;
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
    apply_pending_pricing(&mut deployment, now);
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
    charge_deployment(
        deployment,
        current_base_burn_rate(deployment).saturating_mul(elapsed_seconds),
    );
    deployment.started_at = now;
    if deployment.status == DeploymentStatus::Paused as u8 && deployment.spent >= deployment.deposit
    {
        deployment.paused_at = now;
    }
}

fn apply_reported_usage_burn(
    deployment: &mut Deployment,
    now: i64,
    cpu_cores_used: u32,
    memory_bytes_used: u64,
    storage_bytes_used: u64,
    network_bytes_sent: u64,
) {
    if deployment.status != DeploymentStatus::Running as u8 {
        return;
    }

    let usage_start = if deployment.last_report_at > 0 {
        deployment.last_report_at
    } else {
        deployment.started_at
    };
    if usage_start <= 0 || now <= usage_start {
        return;
    }

    let elapsed_seconds = (now - usage_start) as u64;
    let network_mbits = average_network_mbits(network_bytes_sent, elapsed_seconds);
    let prices = active_prices(deployment);
    let usage_rate = usage_burn_rate_with_prices(
        prices,
        cpu_cores_used,
        memory_bytes_used,
        storage_bytes_used,
        network_mbits,
    );
    charge_deployment(deployment, usage_rate.saturating_mul(elapsed_seconds));
}

fn charge_deployment(deployment: &mut Deployment, amount: u64) {
    deployment.spent = deployment
        .spent
        .saturating_add(amount)
        .min(deployment.deposit);

    if deployment.spent >= deployment.deposit {
        deployment.status = DeploymentStatus::Paused as u8;
    }
}

fn average_network_mbits(network_bytes_sent: u64, elapsed_seconds: u64) -> u64 {
    if network_bytes_sent == 0 || elapsed_seconds == 0 {
        return 0;
    }
    let bits = (network_bytes_sent as u128).saturating_mul(8);
    let denominator = (elapsed_seconds as u128).saturating_mul(1_000_000);
    bits.div_ceil(denominator).min(u64::MAX as u128) as u64
}

fn usage_burn_rate(
    cpu_cores_used: u32,
    memory_bytes_used: u64,
    storage_bytes_used: u64,
    network_mbits: u64,
) -> u64 {
    usage_burn_rate_with_prices(
        (CORE_HOUR, RAM_GIB_HOUR, STORAGE_GIB_HOUR, NETWORK_MBIT_HOUR),
        cpu_cores_used,
        memory_bytes_used,
        storage_bytes_used,
        network_mbits,
    )
}

fn usage_burn_rate_with_prices(
    prices: (u64, u64, u64, u64),
    cpu_cores_used: u32,
    memory_bytes_used: u64,
    storage_bytes_used: u64,
    network_mbits: u64,
) -> u64 {
    let memory_gib = memory_bytes_used.div_ceil(GIB);
    let storage_gib = storage_bytes_used.div_ceil(GIB);
    let hourly = prices
        .0
        .saturating_mul(cpu_cores_used as u64)
        .saturating_add(prices.1.saturating_mul(memory_gib))
        .saturating_add(prices.2.saturating_mul(storage_gib))
        .saturating_add(prices.3.saturating_mul(network_mbits));
    hourly.div_ceil(SECONDS_PER_HOUR)
}

fn active_prices(deployment: &Deployment) -> (u64, u64, u64, u64) {
    (
        if deployment.current_core_hour == 0 {
            CORE_HOUR
        } else {
            deployment.current_core_hour
        },
        if deployment.current_ram_gib_hour == 0 {
            RAM_GIB_HOUR
        } else {
            deployment.current_ram_gib_hour
        },
        if deployment.current_storage_gib_hour == 0 {
            STORAGE_GIB_HOUR
        } else {
            deployment.current_storage_gib_hour
        },
        if deployment.current_network_mbit_hour == 0 {
            NETWORK_MBIT_HOUR
        } else {
            deployment.current_network_mbit_hour
        },
    )
}

fn current_base_burn_rate(deployment: &Deployment) -> u64 {
    if deployment.current_core_hour == 0
        && deployment.current_ram_gib_hour == 0
        && deployment.current_storage_gib_hour == 0
        && deployment.current_network_mbit_hour == 0
    {
        return deployment.burn_rate;
    }

    usage_burn_rate_with_prices(
        active_prices(deployment),
        deployment.total_cpu_cores,
        deployment.total_memory_bytes,
        deployment.total_storage_bytes,
        deployment.total_network_mbps as u64,
    )
}

fn pending_price_increases(deployment: &Deployment) -> bool {
    let current = active_prices(deployment);
    deployment.pending_core_hour > current.0
        || deployment.pending_ram_gib_hour > current.1
        || deployment.pending_storage_gib_hour > current.2
        || deployment.pending_network_mbit_hour > current.3
}

fn apply_pending_pricing(deployment: &mut Deployment, now: i64) {
    if deployment.pending_price_effective_at <= 0 || now < deployment.pending_price_effective_at {
        return;
    }

    let effective_at = deployment.pending_price_effective_at;
    if deployment.status == DeploymentStatus::Running as u8
        && deployment.started_at > 0
        && effective_at > deployment.started_at
    {
        let elapsed_seconds = (effective_at - deployment.started_at) as u64;
        charge_deployment(
            deployment,
            current_base_burn_rate(deployment).saturating_mul(elapsed_seconds),
        );
        deployment.started_at = effective_at;
    }

    if pending_price_increases(deployment) && deployment.auto_stop_on_price_increase != 0 {
        deployment.status = DeploymentStatus::Stopped as u8;
        deployment.paused_at = effective_at;
        deployment.pending_price_effective_at = 0;
        return;
    }

    deployment.current_core_hour = deployment.pending_core_hour;
    deployment.current_ram_gib_hour = deployment.pending_ram_gib_hour;
    deployment.current_storage_gib_hour = deployment.pending_storage_gib_hour;
    deployment.current_network_mbit_hour = deployment.pending_network_mbit_hour;
    deployment.burn_rate = current_base_burn_rate(deployment);
    deployment.pending_core_hour = 0;
    deployment.pending_ram_gib_hour = 0;
    deployment.pending_storage_gib_hour = 0;
    deployment.pending_network_mbit_hour = 0;
    deployment.pending_price_effective_at = 0;
}

fn apply_elapsed_burn_if_running(deployment: &mut Deployment) -> ProgramResult {
    if deployment.status == DeploymentStatus::Running as u8 {
        let now = Clock::get()?.unix_timestamp;
        apply_elapsed_burn(deployment, now);
    }
    Ok(())
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
    let provider_authority = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !provider_authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if deployment.provider != *provider_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }
    {
        let provider_data = provider_account.try_borrow_data()?;
        if provider_data.len() < 32 {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if provider_data[0..32] != *provider_authority.key.as_ref() {
            return Err(DeploymentError::Unauthorized.into());
        }
    }

    let now = Clock::get()?.unix_timestamp;
    apply_pending_pricing(&mut deployment, now);
    if deployment.status == DeploymentStatus::Stopped as u8 {
        deployment.pack(&mut data)?;
        return Ok(());
    }
    apply_elapsed_burn(&mut deployment, now);
    apply_reported_usage_burn(
        &mut deployment,
        now,
        cpu_cores_used,
        memory_bytes_used,
        storage_bytes_used,
        network_bytes_sent,
    );
    if deployment.status == DeploymentStatus::Paused as u8 && deployment.spent >= deployment.deposit
    {
        deployment.paused_at = now;
    }
    deployment.last_report_at = now;
    deployment.cpu_cores_used = cpu_cores_used;
    deployment.memory_bytes_used = memory_bytes_used;
    deployment.storage_bytes_used = storage_bytes_used;
    deployment.network_bytes_sent = network_bytes_sent;

    deployment.pack(&mut data)?;
    Ok(())
}

fn schedule_pricing(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    core_hour: u64,
    ram_gib_hour: u64,
    storage_gib_hour: u64,
    network_mbit_hour: u64,
    effective_at: i64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let governance_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !governance_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if governance_authority(&deployment) != *governance_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    let now = Clock::get()?.unix_timestamp;
    if core_hour == 0 || ram_gib_hour == 0 || storage_gib_hour == 0 || network_mbit_hour == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }
    if effective_at < now.saturating_add(PRICE_CHANGE_GRACE_SECONDS) {
        return Err(DeploymentError::PriceChangeGracePeriod.into());
    }

    deployment.pending_core_hour = core_hour;
    deployment.pending_ram_gib_hour = ram_gib_hour;
    deployment.pending_storage_gib_hour = storage_gib_hour;
    deployment.pending_network_mbit_hour = network_mbit_hour;
    deployment.pending_price_effective_at = effective_at;
    deployment.pack(&mut data)?;
    Ok(())
}

fn assign_provider(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    provider: [u8; 32],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let deployment_account = next_account_info(account_iter)?;
    let scheduler_account = next_account_info(account_iter)?;
    let provider_account = next_account_info(account_iter)?;

    require_program_owned(deployment_account, program_id)?;

    if !scheduler_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let provider = Pubkey::new_from_array(provider);
    if provider == Pubkey::default() || provider != *provider_account.key {
        return Err(ProgramError::InvalidArgument);
    }

    let mut data = deployment_account.try_borrow_mut_data()?;
    let mut deployment = Deployment::unpack(&data)?;

    if governance_authority(&deployment) != *scheduler_account.key {
        return Err(DeploymentError::Unauthorized.into());
    }

    if deployment.status != DeploymentStatus::Created as u8 {
        return Err(DeploymentError::InvalidState.into());
    }

    {
        let provider_data = provider_account.try_borrow_data()?;
        if provider_data.len() < 81 {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if provider_data[80] != 0 {
            return Err(DeploymentError::InvalidState.into());
        }
        if provider_data[0..32] == *deployment.owner.as_ref() {
            return Err(DeploymentError::SelfProviderNotAllowed.into());
        }
    }

    deployment.provider = provider;
    deployment.pack(&mut data)?;
    Ok(())
}

fn require_program_owned(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
    if account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

fn governance_authority(deployment: &Deployment) -> Pubkey {
    if deployment.governance_authority == Pubkey::default() {
        deployment.owner
    } else {
        deployment.governance_authority
    }
}

fn uses_token_escrow(deployment: &Deployment) -> bool {
    deployment.payment_mint != Pubkey::default()
        && deployment.escrow_token_account != Pubkey::default()
}

fn escrow_authority(program_id: &Pubkey, deployment: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ESCROW_AUTHORITY_SEED, deployment.as_ref()], program_id)
}

fn require_uninitialized_deployment(data: &[u8]) -> ProgramResult {
    let deployment = Deployment::unpack(data)?;
    if deployment.owner != Pubkey::default()
        || deployment.deposit != 0
        || deployment.created_at != 0
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}

fn validate_token_account(
    account: &AccountInfo,
    token_program: &Pubkey,
    expected_mint: &Pubkey,
    expected_owner: &Pubkey,
) -> ProgramResult {
    validate_token_account_mint(account, token_program, expected_mint)?;
    let data = account.try_borrow_data()?;
    if &data[TOKEN_ACCOUNT_OWNER_OFFSET..TOKEN_ACCOUNT_OWNER_OFFSET + 32] != expected_owner.as_ref()
    {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

fn validate_token_account_mint(
    account: &AccountInfo,
    token_program: &Pubkey,
    expected_mint: &Pubkey,
) -> ProgramResult {
    if account.owner != token_program {
        return Err(ProgramError::IncorrectProgramId);
    }
    let data = account.try_borrow_data()?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    if &data[TOKEN_ACCOUNT_MINT_OFFSET..TOKEN_ACCOUNT_MINT_OFFSET + 32] != expected_mint.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

fn transfer_spl_tokens<'a>(
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Ok(());
    }
    invoke(
        &token_transfer_instruction(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            amount,
        ),
        &[
            source.clone(),
            destination.clone(),
            authority.clone(),
            token_program.clone(),
        ],
    )
}

fn transfer_from_escrow<'a>(
    program_id: &Pubkey,
    deployment_account: &AccountInfo<'a>,
    deployment: &Deployment,
    escrow_token_account: &AccountInfo<'a>,
    escrow_authority_account: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Ok(());
    }
    if *escrow_token_account.key != deployment.escrow_token_account {
        return Err(ProgramError::InvalidArgument);
    }
    let (authority, bump) = escrow_authority(program_id, deployment_account.key);
    if *escrow_authority_account.key != authority {
        return Err(ProgramError::InvalidArgument);
    }
    validate_token_account(
        escrow_token_account,
        token_program.key,
        &deployment.payment_mint,
        &authority,
    )?;
    validate_token_account_mint(destination, token_program.key, &deployment.payment_mint)?;
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[
        ESCROW_AUTHORITY_SEED,
        deployment_account.key.as_ref(),
        &bump_seed,
    ];
    invoke_signed(
        &token_transfer_instruction(
            token_program.key,
            escrow_token_account.key,
            destination.key,
            &authority,
            amount,
        ),
        &[
            escrow_token_account.clone(),
            destination.clone(),
            escrow_authority_account.clone(),
            token_program.clone(),
        ],
        &[seeds],
    )
}

fn token_transfer_instruction(
    token_program: &Pubkey,
    source: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_TRANSFER_INSTRUCTION);
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: *token_program,
        accounts: vec![
            AccountMeta::new(*source, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
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
        let provider_authority = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut deployment_lamports = 0;
        let mut provider_lamports = 0;
        let mut authority_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut provider_data = [0u8; 128];
        let mut authority_data = [];
        provider_data[0..32].copy_from_slice(provider_authority.as_ref());

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
            ..state::Deployment::default()
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
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let authority_account = account(
            &provider_authority,
            &program_id,
            true,
            &mut authority_data,
            &mut authority_lamports,
        );
        let accounts = vec![deployment_account, provider_account, authority_account];
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
    fn start_rejects_unassigned_provider() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut deployment_lamports = 0;
        let mut owner_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut owner_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::default(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1000,
            burn_rate: 1,
            status: state::DeploymentStatus::Created as u8,
            ..state::Deployment::default()
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
            Err(DeploymentError::ProviderNotAssigned.into())
        );
    }

    #[test]
    fn assign_provider_requires_scheduler_and_rejects_self_provider() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut deployment_lamports = 0;
        let mut scheduler_lamports = 0;
        let mut provider_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut scheduler_data = [];
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(owner.as_ref());

        state::Deployment {
            owner,
            provider: Pubkey::default(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1000,
            burn_rate: 1,
            status: state::DeploymentStatus::Created as u8,
            ..state::Deployment::default()
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
        let scheduler_account = account(
            &owner,
            &program_id,
            true,
            &mut scheduler_data,
            &mut scheduler_lamports,
        );
        let provider_account = account(
            &provider_key,
            &program_id,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let accounts = vec![deployment_account, scheduler_account, provider_account];
        let mut instruction = vec![10];
        instruction.extend_from_slice(provider_key.as_ref());

        assert_eq!(
            process_instruction(&program_id, &accounts, &instruction),
            Err(DeploymentError::SelfProviderNotAllowed.into())
        );
    }

    #[test]
    fn assign_provider_sets_active_provider() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let provider_authority = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let governance = Pubkey::new_unique();
        let mut deployment_lamports = 0;
        let mut scheduler_lamports = 0;
        let mut provider_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut scheduler_data = [];
        let mut provider_data = [0u8; 128];
        provider_data[0..32].copy_from_slice(provider_authority.as_ref());

        state::Deployment {
            owner,
            provider: Pubkey::default(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1000,
            burn_rate: 1,
            status: state::DeploymentStatus::Created as u8,
            governance_authority: governance,
            ..state::Deployment::default()
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
        let scheduler_account = account(
            &governance,
            &program_id,
            true,
            &mut scheduler_data,
            &mut scheduler_lamports,
        );
        let provider_account = account(
            &provider_key,
            &program_id,
            false,
            &mut provider_data,
            &mut provider_lamports,
        );
        let accounts = vec![deployment_account, scheduler_account, provider_account];
        let mut instruction = vec![10];
        instruction.extend_from_slice(provider_key.as_ref());

        process_instruction(&program_id, &accounts, &instruction).unwrap();

        let deployment = state::Deployment::unpack(&deployment_data).unwrap();
        assert_eq!(deployment.provider, provider_key);
    }

    #[test]
    fn resolve_requires_governance_authority() {
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
            ..state::Deployment::default()
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
    fn resolve_accepts_configured_governance_authority() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let governance = Pubkey::new_unique();
        let refund_key = Pubkey::new_unique();
        let provider_payout_key = Pubkey::new_unique();
        let slash_key = Pubkey::new_unique();
        let mut deployment_lamports = 1_000;
        let mut governance_lamports = 0;
        let mut refund_lamports = 0;
        let mut provider_payout_lamports = 0;
        let mut slash_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut governance_data = [];
        let mut refund_data = [];
        let mut provider_payout_data = [];
        let mut slash_data = [];

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
            spent: 100,
            status: state::DeploymentStatus::Disputed as u8,
            governance_authority: governance,
            ..state::Deployment::default()
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
        let governance_account = account(
            &governance,
            &program_id,
            true,
            &mut governance_data,
            &mut governance_lamports,
        );
        let refund_account = account(
            &refund_key,
            &program_id,
            false,
            &mut refund_data,
            &mut refund_lamports,
        );
        let provider_payout_account = account(
            &provider_payout_key,
            &program_id,
            false,
            &mut provider_payout_data,
            &mut provider_payout_lamports,
        );
        let slash_account = account(
            &slash_key,
            &program_id,
            false,
            &mut slash_data,
            &mut slash_lamports,
        );
        let accounts = vec![
            deployment_account,
            governance_account,
            refund_account,
            provider_payout_account,
            slash_account,
        ];
        let mut instruction = vec![6];
        instruction.extend_from_slice(&400u64.to_le_bytes());
        instruction.extend_from_slice(&100u64.to_le_bytes());
        instruction.extend_from_slice(&500u64.to_le_bytes());

        process_instruction(&program_id, &accounts, &instruction).unwrap();

        let deployment = state::Deployment::unpack(&deployment_data).unwrap();
        assert_eq!(deployment.status, state::DeploymentStatus::Stopped as u8);
        assert_eq!(deployment.spent, 600);
        assert_eq!(deployment.provider_earned, 100);
        assert_eq!(deployment.buyer_refunded, 400);
        assert_eq!(deployment.dao_slashed, 500);
        assert_eq!(refund_lamports, 400);
        assert_eq!(provider_payout_lamports, 100);
        assert_eq!(slash_lamports, 500);
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
            ..state::Deployment::default()
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
            ..state::Deployment::default()
        };

        apply_elapsed_burn(&mut deployment, 12);

        assert_eq!(deployment.spent, 100);
        assert_eq!(deployment.started_at, 12);
        assert_eq!(deployment.paused_at, 12);
        assert_eq!(deployment.status, state::DeploymentStatus::Paused as u8);
    }

    #[test]
    fn report_usage_burns_elapsed_usage_interval() {
        let mut deployment = state::Deployment {
            owner: Pubkey::new_unique(),
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 10_000,
            burn_rate: 0,
            spent: 10,
            status: state::DeploymentStatus::Running as u8,
            created_at: 0,
            started_at: 100,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 100,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
            ..state::Deployment::default()
        };

        apply_reported_usage_burn(&mut deployment, 110, 1, GIB, GIB, 1_250_000);

        assert_eq!(average_network_mbits(1_250_000, 10), 1);
        assert_eq!(usage_burn_rate(1, GIB, GIB, 1), 5);
        assert_eq!(deployment.spent, 60);
        assert_eq!(deployment.status, state::DeploymentStatus::Running as u8);
    }

    #[test]
    fn report_usage_caps_at_deposit_and_pauses() {
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
            burn_rate: 0,
            spent: 90,
            status: state::DeploymentStatus::Running as u8,
            created_at: 0,
            started_at: 100,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 100,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
            ..state::Deployment::default()
        };

        apply_reported_usage_burn(&mut deployment, 200, 100, 100 * GIB, 100 * GIB, 0);

        assert_eq!(deployment.spent, 100);
        assert_eq!(deployment.status, state::DeploymentStatus::Paused as u8);
    }

    #[test]
    fn pending_price_increase_auto_stops_after_old_price_grace_burn() {
        let mut deployment = state::Deployment {
            owner: Pubkey::new_unique(),
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 1,
            total_memory_bytes: 0,
            total_storage_bytes: 0,
            total_network_mbps: 0,
            deposit: 1_000,
            burn_rate: 1,
            spent: 10,
            status: state::DeploymentStatus::Running as u8,
            created_at: 0,
            started_at: 100,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 100,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
            current_core_hour: 3_600,
            current_ram_gib_hour: 1,
            current_storage_gib_hour: 1,
            current_network_mbit_hour: 1,
            pending_core_hour: 7_200,
            pending_ram_gib_hour: 1,
            pending_storage_gib_hour: 1,
            pending_network_mbit_hour: 1,
            pending_price_effective_at: 200,
            auto_stop_on_price_increase: 1,
            ..state::Deployment::default()
        };

        apply_pending_pricing(&mut deployment, 201);

        assert_eq!(deployment.spent, 110);
        assert_eq!(deployment.started_at, 200);
        assert_eq!(deployment.paused_at, 200);
        assert_eq!(deployment.status, state::DeploymentStatus::Stopped as u8);
        assert_eq!(deployment.pending_price_effective_at, 0);
        assert_eq!(deployment.current_core_hour, 3_600);
    }

    #[test]
    fn pending_price_increase_can_be_accepted_by_opt_out() {
        let mut deployment = state::Deployment {
            owner: Pubkey::new_unique(),
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 1,
            total_memory_bytes: 0,
            total_storage_bytes: 0,
            total_network_mbps: 0,
            deposit: 1_000,
            burn_rate: 1,
            spent: 10,
            status: state::DeploymentStatus::Running as u8,
            created_at: 0,
            started_at: 100,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 100,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
            current_core_hour: 3_600,
            current_ram_gib_hour: 1,
            current_storage_gib_hour: 1,
            current_network_mbit_hour: 1,
            pending_core_hour: 7_200,
            pending_ram_gib_hour: 1,
            pending_storage_gib_hour: 1,
            pending_network_mbit_hour: 1,
            pending_price_effective_at: 200,
            auto_stop_on_price_increase: 0,
            ..state::Deployment::default()
        };

        apply_pending_pricing(&mut deployment, 201);

        assert_eq!(deployment.spent, 110);
        assert_eq!(deployment.started_at, 200);
        assert_eq!(deployment.status, state::DeploymentStatus::Running as u8);
        assert_eq!(deployment.current_core_hour, 7_200);
        assert_eq!(deployment.burn_rate, 2);
        assert_eq!(deployment.pending_price_effective_at, 0);
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
            ..state::Deployment::default()
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

    #[test]
    fn initialize_rejects_existing_deployment_state_before_funding() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let system_key = system_program_id();
        let mut deployment_lamports = 0;
        let mut owner_lamports = 1_000;
        let mut system_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut owner_data = [];
        let mut system_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 100,
            burn_rate: 1,
            spent: 0,
            status: state::DeploymentStatus::Created as u8,
            created_at: 1,
            started_at: 0,
            paused_at: 0,
            bump_seed: 0,
            last_report_at: 0,
            cpu_cores_used: 0,
            memory_bytes_used: 0,
            storage_bytes_used: 0,
            network_bytes_sent: 0,
            ..state::Deployment::default()
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
        let owner_account = account(
            &owner,
            &program_id,
            true,
            &mut owner_data,
            &mut owner_lamports,
        );
        let system_account = account(
            &system_key,
            &system_key,
            false,
            &mut system_data,
            &mut system_lamports,
        );
        let accounts = vec![deployment_account, owner_account, system_account];
        let mut instruction = vec![0];
        instruction.extend_from_slice(&[0u8; 64]);
        instruction.extend_from_slice(Pubkey::new_unique().as_ref());
        instruction.extend_from_slice(&1u32.to_le_bytes());
        instruction.extend_from_slice(&2u32.to_le_bytes());
        instruction.extend_from_slice(&4096u64.to_le_bytes());
        instruction.extend_from_slice(&8192u64.to_le_bytes());
        instruction.extend_from_slice(&100u32.to_le_bytes());
        instruction.extend_from_slice(&100u64.to_le_bytes());
        instruction.extend_from_slice(&1u64.to_le_bytes());

        assert_eq!(
            process_instruction(&program_id, &accounts, &instruction),
            Err(ProgramError::AccountAlreadyInitialized)
        );
        assert_eq!(deployment_lamports, 0);
        assert_eq!(owner_lamports, 1_000);
    }

    #[test]
    fn stop_rejects_disputed_or_already_stopped_deployment() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let provider_payout_key = Pubkey::new_unique();
        let mut deployment_lamports = 1_000;
        let mut owner_lamports = 0;
        let mut provider_payout_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut owner_data = [];
        let mut provider_payout_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1_000,
            burn_rate: 1,
            spent: 200,
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
            ..state::Deployment::default()
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
        let owner_account = account(
            &owner,
            &program_id,
            true,
            &mut owner_data,
            &mut owner_lamports,
        );
        let provider_payout_account = account(
            &provider_payout_key,
            &program_id,
            false,
            &mut provider_payout_data,
            &mut provider_payout_lamports,
        );
        let accounts = vec![deployment_account, owner_account, provider_payout_account];

        assert_eq!(
            process_instruction(&program_id, &accounts, &[4]),
            Err(DeploymentError::InvalidState.into())
        );
        assert_eq!(deployment_lamports, 1_000);
        assert_eq!(owner_lamports, 0);
    }

    #[test]
    fn dispute_and_resolve_enforce_lifecycle_state() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let refund_key = Pubkey::new_unique();
        let slash_key = Pubkey::new_unique();
        let mut deployment_lamports = 1_000;
        let mut owner_lamports = 0;
        let mut refund_lamports = 0;
        let mut slash_lamports = 0;
        let mut deployment_data = [0u8; state::SIZE];
        let mut owner_data = [];
        let mut refund_data = [];
        let mut slash_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1_000,
            burn_rate: 1,
            spent: 200,
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
            ..state::Deployment::default()
        }
        .pack(&mut deployment_data)
        .unwrap();

        {
            let deployment_account = account(
                &deployment_key,
                &program_id,
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
                process_instruction(&program_id, &accounts, &[5]),
                Err(DeploymentError::InvalidState.into())
            );
        }

        {
            let deployment_account = account(
                &deployment_key,
                &program_id,
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
            let refund_account = account(
                &refund_key,
                &program_id,
                false,
                &mut refund_data,
                &mut refund_lamports,
            );
            let slash_account = account(
                &slash_key,
                &program_id,
                false,
                &mut slash_data,
                &mut slash_lamports,
            );
            let accounts = vec![
                deployment_account,
                owner_account,
                refund_account,
                slash_account,
            ];
            let mut instruction = vec![6];
            instruction.extend_from_slice(&100u64.to_le_bytes());
            instruction.extend_from_slice(&0u64.to_le_bytes());
            assert_eq!(
                process_instruction(&program_id, &accounts, &instruction),
                Err(DeploymentError::InvalidState.into())
            );
        }

        assert_eq!(deployment_lamports, 1_000);
        assert_eq!(refund_lamports, 0);
        assert_eq!(slash_lamports, 0);
    }

    #[test]
    fn lamport_transfer_moves_escrow_balance() {
        let program_id = Pubkey::new_unique();
        let source_key = Pubkey::new_unique();
        let dest_key = Pubkey::new_unique();
        let mut source_lamports = 1_000;
        let mut dest_lamports = 10;
        let mut source_data = [];
        let mut dest_data = [];
        let source = account(
            &source_key,
            &program_id,
            true,
            &mut source_data,
            &mut source_lamports,
        );
        let dest = account(
            &dest_key,
            &program_id,
            false,
            &mut dest_data,
            &mut dest_lamports,
        );

        transfer_lamports(&source, &dest, 250).unwrap();

        assert_eq!(source.lamports(), 750);
        assert_eq!(dest.lamports(), 260);
    }

    #[test]
    fn lamport_transfer_rejects_insufficient_funds() {
        let program_id = Pubkey::new_unique();
        let source_key = Pubkey::new_unique();
        let dest_key = Pubkey::new_unique();
        let mut source_lamports = 5;
        let mut dest_lamports = 10;
        let mut source_data = [];
        let mut dest_data = [];
        let source = account(
            &source_key,
            &program_id,
            true,
            &mut source_data,
            &mut source_lamports,
        );
        let dest = account(
            &dest_key,
            &program_id,
            false,
            &mut dest_data,
            &mut dest_lamports,
        );

        assert_eq!(
            transfer_lamports(&source, &dest, 250),
            Err(ProgramError::InsufficientFunds)
        );
        assert_eq!(source.lamports(), 5);
        assert_eq!(dest.lamports(), 10);
    }

    #[test]
    fn resolve_distributes_remaining_escrow() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let refund_key = Pubkey::new_unique();
        let provider_payout_key = Pubkey::new_unique();
        let slash_key = Pubkey::new_unique();
        let mut deployment_lamports = 1_000;
        let mut owner_lamports = 0;
        let mut refund_lamports = 10;
        let mut provider_payout_lamports = 30;
        let mut slash_lamports = 20;
        let mut deployment_data = [0u8; state::SIZE];
        let mut owner_data = [];
        let mut refund_data = [];
        let mut provider_payout_data = [];
        let mut slash_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1_000,
            burn_rate: 1,
            spent: 200,
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
            ..state::Deployment::default()
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
        let owner_account = account(
            &owner,
            &program_id,
            true,
            &mut owner_data,
            &mut owner_lamports,
        );
        let refund_account = account(
            &refund_key,
            &program_id,
            false,
            &mut refund_data,
            &mut refund_lamports,
        );
        let provider_payout_account = account(
            &provider_payout_key,
            &program_id,
            false,
            &mut provider_payout_data,
            &mut provider_payout_lamports,
        );
        let slash_account = account(
            &slash_key,
            &program_id,
            false,
            &mut slash_data,
            &mut slash_lamports,
        );
        let accounts = vec![
            deployment_account,
            owner_account,
            refund_account,
            provider_payout_account,
            slash_account,
        ];
        let mut instruction = vec![6];
        instruction.extend_from_slice(&500u64.to_le_bytes());
        instruction.extend_from_slice(&200u64.to_le_bytes());
        instruction.extend_from_slice(&300u64.to_le_bytes());

        process_instruction(&program_id, &accounts, &instruction).unwrap();

        let deployment = state::Deployment::unpack(&deployment_data).unwrap();
        assert_eq!(deployment_lamports, 0);
        assert_eq!(refund_lamports, 510);
        assert_eq!(provider_payout_lamports, 230);
        assert_eq!(slash_lamports, 320);
        assert_eq!(deployment.status, state::DeploymentStatus::Stopped as u8);
        assert_eq!(deployment.spent, 500);
        assert_eq!(deployment.provider_earned, 200);
        assert_eq!(deployment.buyer_refunded, 500);
        assert_eq!(deployment.dao_slashed, 300);
    }

    #[test]
    fn resolve_rejects_over_settlement() {
        let program_id = Pubkey::new_unique();
        let deployment_key = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let refund_key = Pubkey::new_unique();
        let provider_payout_key = Pubkey::new_unique();
        let slash_key = Pubkey::new_unique();
        let mut deployment_lamports = 1_000;
        let mut owner_lamports = 0;
        let mut refund_lamports = 10;
        let mut provider_payout_lamports = 30;
        let mut slash_lamports = 20;
        let mut deployment_data = [0u8; state::SIZE];
        let mut owner_data = [];
        let mut refund_data = [];
        let mut provider_payout_data = [];
        let mut slash_data = [];

        state::Deployment {
            owner,
            provider: Pubkey::new_unique(),
            name: [0u8; 64],
            container_count: 1,
            total_cpu_cores: 2,
            total_memory_bytes: 4096,
            total_storage_bytes: 8192,
            total_network_mbps: 100,
            deposit: 1_000,
            burn_rate: 1,
            spent: 200,
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
            ..state::Deployment::default()
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
        let owner_account = account(
            &owner,
            &program_id,
            true,
            &mut owner_data,
            &mut owner_lamports,
        );
        let refund_account = account(
            &refund_key,
            &program_id,
            false,
            &mut refund_data,
            &mut refund_lamports,
        );
        let provider_payout_account = account(
            &provider_payout_key,
            &program_id,
            false,
            &mut provider_payout_data,
            &mut provider_payout_lamports,
        );
        let slash_account = account(
            &slash_key,
            &program_id,
            false,
            &mut slash_data,
            &mut slash_lamports,
        );
        let accounts = vec![
            deployment_account,
            owner_account,
            refund_account,
            provider_payout_account,
            slash_account,
        ];
        let mut instruction = vec![6];
        instruction.extend_from_slice(&801u64.to_le_bytes());
        instruction.extend_from_slice(&200u64.to_le_bytes());
        instruction.extend_from_slice(&0u64.to_le_bytes());

        assert_eq!(
            process_instruction(&program_id, &accounts, &instruction),
            Err(ProgramError::InsufficientFunds)
        );
        assert_eq!(deployment_lamports, 1_000);
        assert_eq!(refund_lamports, 10);
        assert_eq!(provider_payout_lamports, 30);
        assert_eq!(slash_lamports, 20);
    }
}
