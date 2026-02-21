// SPDX-License-Identifier: Apache-2.0
#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::ed25519_program;
use anchor_lang::solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked,
};
use anchor_lang::system_program::{self, Transfer};
use std::collections::{HashMap, HashSet};

declare_id!("AgjxA2CoMmmWXrcsJtvvpmqdRHLVHrhYf6DAuBCL4s5T");

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ConfigParams {
    pub scheduler_authority: Pubkey,
    pub randomness_authority: Pubkey,
    pub min_worker_stake_lamports: u64,
    pub protocol_fee_bps: u16,
    pub challenge_window_slots: u64,
    pub da_window_slots: u64,
    pub non_response_slash_lamports: u64,
    pub committee_tiering_enabled: bool,
    pub max_committee_size: u8,
    pub max_memory_bytes: u32,
    pub max_instructions: u64,
    pub allowed_runtime_root: [u8; 32],
    pub paused: bool,
}

#[program]
pub mod edgerun_program {
    use super::*;

    pub fn initialize_config(ctx: Context<InitializeConfig>, params: ConfigParams) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.scheduler_authority = params.scheduler_authority;
        config.randomness_authority = params.randomness_authority;
        config.min_worker_stake_lamports = params.min_worker_stake_lamports;
        config.protocol_fee_bps = params.protocol_fee_bps;
        config.challenge_window_slots = params.challenge_window_slots;
        config.da_window_slots = params.da_window_slots;
        config.non_response_slash_lamports = params.non_response_slash_lamports;
        config.committee_tiering_enabled = params.committee_tiering_enabled;
        config.max_committee_size = params.max_committee_size.max(3);
        config.max_memory_bytes = params.max_memory_bytes;
        config.max_instructions = params.max_instructions;
        config.allowed_runtime_root = params.allowed_runtime_root;
        config.committee_size = 3;
        config.quorum = 2;
        config.paused = params.paused;

        Ok(())
    }

    pub fn update_config(ctx: Context<UpdateConfig>, params: ConfigParams) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.scheduler_authority = params.scheduler_authority;
        config.randomness_authority = params.randomness_authority;
        config.min_worker_stake_lamports = params.min_worker_stake_lamports;
        config.protocol_fee_bps = params.protocol_fee_bps;
        config.challenge_window_slots = params.challenge_window_slots;
        config.da_window_slots = params.da_window_slots;
        config.non_response_slash_lamports = params.non_response_slash_lamports;
        config.committee_tiering_enabled = params.committee_tiering_enabled;
        config.max_committee_size = params.max_committee_size.max(3);
        config.max_memory_bytes = params.max_memory_bytes;
        config.max_instructions = params.max_instructions;
        config.allowed_runtime_root = params.allowed_runtime_root;
        config.paused = params.paused;
        Ok(())
    }

    pub fn register_worker_stake(ctx: Context<RegisterWorkerStake>) -> Result<()> {
        let stake = &mut ctx.accounts.worker_stake;
        stake.worker = ctx.accounts.worker.key();
        stake.total_stake_lamports = 0;
        stake.locked_stake_lamports = 0;
        stake.status = WorkerStatus::Active as u8;
        Ok(())
    }

    pub fn deposit_stake(ctx: Context<DepositStake>, amount_lamports: u64) -> Result<()> {
        require!(amount_lamports > 0, EdgerunError::InvalidAmount);

        let transfer_accounts = Transfer {
            from: ctx.accounts.worker.to_account_info(),
            to: ctx.accounts.worker_stake.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );
        system_program::transfer(transfer_ctx, amount_lamports)?;

        let stake = &mut ctx.accounts.worker_stake;
        stake.total_stake_lamports = stake
            .total_stake_lamports
            .checked_add(amount_lamports)
            .ok_or(EdgerunError::MathOverflow)?;

        Ok(())
    }

    pub fn withdraw_stake(ctx: Context<WithdrawStake>, amount_lamports: u64) -> Result<()> {
        require!(amount_lamports > 0, EdgerunError::InvalidAmount);

        let stake = &ctx.accounts.worker_stake;
        require!(
            stake.status == WorkerStatus::Active as u8,
            EdgerunError::WorkerNotActive
        );

        let post_withdraw_total = stake
            .total_stake_lamports
            .checked_sub(amount_lamports)
            .ok_or(EdgerunError::InsufficientStake)?;

        require!(
            post_withdraw_total >= stake.locked_stake_lamports,
            EdgerunError::StakeLocked
        );

        let rent_floor = Rent::get()?.minimum_balance(8 + WorkerStake::INIT_SPACE);
        let post_withdraw_lamports = ctx
            .accounts
            .worker_stake
            .to_account_info()
            .lamports()
            .checked_sub(amount_lamports)
            .ok_or(EdgerunError::InsufficientVaultLamports)?;
        require!(
            post_withdraw_lamports >= rent_floor,
            EdgerunError::RentExemptionViolation
        );

        transfer_lamports_from_program_owned(
            &ctx.accounts.worker_stake.to_account_info(),
            &ctx.accounts.worker.to_account_info(),
            amount_lamports,
        )?;

        let stake = &mut ctx.accounts.worker_stake;
        stake.total_stake_lamports = post_withdraw_total;

        Ok(())
    }

    pub fn post_job(
        ctx: Context<PostJob>,
        job_id: [u8; 32],
        bundle_hash: [u8; 32],
        runtime_id: [u8; 32],
        runtime_proof: Vec<[u8; 32]>,
        max_memory_bytes: u32,
        max_instructions: u64,
        escrow_lamports: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        require!(!config.paused, EdgerunError::Paused);
        require!(escrow_lamports > 0, EdgerunError::InvalidAmount);
        require!(bundle_hash != [0u8; 32], EdgerunError::InvalidBundleHash);
        require!(runtime_id != [0u8; 32], EdgerunError::InvalidRuntimeId);
        require!(
            max_memory_bytes <= config.max_memory_bytes,
            EdgerunError::LimitExceeded
        );
        require!(
            max_instructions <= config.max_instructions,
            EdgerunError::LimitExceeded
        );
        if config.allowed_runtime_root != [0_u8; 32] {
            require!(
                verify_runtime_membership(
                    &config.allowed_runtime_root,
                    &runtime_id,
                    &runtime_proof
                ),
                EdgerunError::RuntimeNotAllowed
            );
        }

        let transfer_accounts = Transfer {
            from: ctx.accounts.client.to_account_info(),
            to: ctx.accounts.job.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );
        system_program::transfer(transfer_ctx, escrow_lamports)?;

        let job = &mut ctx.accounts.job;
        let (committee_size, quorum) = committee_tier_for_escrow(escrow_lamports);
        job.job_id = job_id;
        job.client = ctx.accounts.client.key();
        job.escrow_lamports = escrow_lamports;
        // Boundary contract: this must be compute_bundle_hash(canonical_bundle_payload_bytes).
        job.bundle_hash = bundle_hash;
        job.runtime_id = runtime_id;
        job.max_memory_bytes = max_memory_bytes;
        job.max_instructions = max_instructions;
        job.committee_size = if config.committee_tiering_enabled {
            committee_size.min(config.max_committee_size)
        } else {
            config.committee_size
        };
        job.quorum = if config.committee_tiering_enabled {
            quorum.min(job.committee_size)
        } else {
            config.quorum
        };
        job.created_slot = Clock::get()?.slot;
        job.deadline_slot = job.created_slot + config.challenge_window_slots;
        job.assigned_workers = [Pubkey::default(); 9];
        job.assigned_count = 0;
        job.required_lock_lamports = 0;
        job.winning_output_hash = [0_u8; 32];
        job.seed = [0_u8; 32];
        job.seed_set = false;
        job.quorum_reached_slot = 0;
        job.da_deadline_slot = 0;
        job.status = JobStatus::Posted as u8;

        Ok(())
    }

    pub fn assign_workers(ctx: Context<AssignWorkers>, workers: [Pubkey; 3]) -> Result<()> {
        let config = &ctx.accounts.config;
        let job = &mut ctx.accounts.job;

        require!(
            job.status == JobStatus::Posted as u8 || job.status == JobStatus::Seeded as u8,
            EdgerunError::InvalidJobState
        );
        if config.committee_tiering_enabled {
            require!(job.seed_set, EdgerunError::SeedNotSet);
        }
        require!(
            workers[0] != workers[1] && workers[0] != workers[2] && workers[1] != workers[2],
            EdgerunError::DuplicateAssignedWorker
        );

        let required_lock = required_lock_for_job(job.escrow_lamports, config.committee_size)
            .max(config.min_worker_stake_lamports);

        lock_worker_for_job(&mut ctx.accounts.worker_stake_0, workers[0], required_lock)?;
        lock_worker_for_job(&mut ctx.accounts.worker_stake_1, workers[1], required_lock)?;
        lock_worker_for_job(&mut ctx.accounts.worker_stake_2, workers[2], required_lock)?;

        job.assigned_workers = [Pubkey::default(); 9];
        job.assigned_workers[0] = workers[0];
        job.assigned_workers[1] = workers[1];
        job.assigned_workers[2] = workers[2];
        job.assigned_count = 3;
        job.required_lock_lamports = required_lock;
        job.status = JobStatus::Assigned as u8;

        Ok(())
    }

    pub fn set_job_seed(ctx: Context<SetJobSeed>, seed: [u8; 32]) -> Result<()> {
        let job = &mut ctx.accounts.job;
        require!(
            job.status == JobStatus::Posted as u8,
            EdgerunError::InvalidJobState
        );
        require!(!job.seed_set, EdgerunError::SeedAlreadySet);
        require!(seed != [0_u8; 32], EdgerunError::InvalidSeed);
        job.seed = seed;
        job.seed_set = true;
        job.status = JobStatus::Seeded as u8;
        Ok(())
    }

    pub fn submit_result(
        ctx: Context<SubmitResult>,
        output_hash: [u8; 32],
        attestation_sig: [u8; 64],
    ) -> Result<()> {
        let job = &ctx.accounts.job;
        require!(
            job.status == JobStatus::Assigned as u8,
            EdgerunError::InvalidJobState
        );
        require!(
            job.assigned_workers.contains(&ctx.accounts.worker.key()),
            EdgerunError::WorkerNotAssigned
        );
        require!(
            Clock::get()?.slot <= job.deadline_slot,
            EdgerunError::JobResultSubmissionExpired
        );
        verify_worker_attestation(
            &ctx.accounts.instructions_sysvar,
            &ctx.accounts.worker.key(),
            &job.job_id,
            &job.bundle_hash,
            &output_hash,
            &job.runtime_id,
            &attestation_sig,
        )?;

        let result = &mut ctx.accounts.job_result;
        result.job_id = job.job_id;
        result.worker = ctx.accounts.worker.key();
        result.output_hash = output_hash;
        result.attestation_sig = attestation_sig;
        result.submitted_slot = Clock::get()?.slot;
        Ok(())
    }

    pub fn finalize_job(ctx: Context<FinalizeJob>) -> Result<()> {
        let job = &ctx.accounts.job;
        require!(
            job.status == JobStatus::Assigned as u8,
            EdgerunError::InvalidJobState
        );
        let finalize_inputs = parse_finalize_inputs(job, &ctx.remaining_accounts)?;
        let winner_count_u64 = finalize_inputs.winner_keys.len() as u64;

        unlock_winner_stakes(
            job.required_lock_lamports,
            &finalize_inputs.winner_keys,
            [
                &mut ctx.accounts.worker_stake_0,
                &mut ctx.accounts.worker_stake_1,
                &mut ctx.accounts.worker_stake_2,
            ],
        )?;

        let escrow_lamports = job.escrow_lamports;
        let protocol_fee = escrow_lamports
            .checked_mul(u64::from(ctx.accounts.config.protocol_fee_bps))
            .ok_or(EdgerunError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(EdgerunError::MathOverflow)?;
        let worker_pool = escrow_lamports
            .checked_sub(protocol_fee)
            .ok_or(EdgerunError::MathOverflow)?;
        let payout_each = worker_pool
            .checked_div(winner_count_u64)
            .ok_or(EdgerunError::MathOverflow)?;
        let payout_remainder = worker_pool
            .checked_rem(winner_count_u64)
            .ok_or(EdgerunError::MathOverflow)?;

        transfer_lamports_from_program_owned(
            &ctx.accounts.job.to_account_info(),
            &ctx.accounts.config.to_account_info(),
            protocol_fee,
        )?;

        for winner in finalize_inputs.winner_accounts.iter() {
            let winner_account = ctx
                .remaining_accounts
                .iter()
                .find(|a| a.key == winner)
                .ok_or(error!(EdgerunError::MissingWinnerPayoutAccount))?;
            transfer_lamports_from_program_owned(
                &ctx.accounts.job.to_account_info(),
                winner_account,
                payout_each,
            )?;
        }
        if payout_remainder > 0 {
            let winner_account = ctx
                .remaining_accounts
                .iter()
                .find(|a| a.key == &finalize_inputs.winner_accounts[0])
                .ok_or(error!(EdgerunError::MissingWinnerPayoutAccount))?;
            transfer_lamports_from_program_owned(
                &ctx.accounts.job.to_account_info(),
                winner_account,
                payout_remainder,
            )?;
        }

        let job = &mut ctx.accounts.job;
        job.winning_output_hash = finalize_inputs.winning_output_hash;
        job.escrow_lamports = 0;
        job.status = JobStatus::Finalized as u8;

        Ok(())
    }

    pub fn cancel_expired_job(ctx: Context<CancelExpiredJob>) -> Result<()> {
        let job = &ctx.accounts.job;
        let current_slot = Clock::get()?.slot;

        require!(
            ctx.accounts.caller.key() == job.client
                || ctx.accounts.caller.key() == ctx.accounts.config.scheduler_authority,
            EdgerunError::UnauthorizedCancelCaller
        );
        require!(
            job.status == JobStatus::Assigned as u8,
            EdgerunError::InvalidJobState
        );
        require!(
            current_slot > job.deadline_slot,
            EdgerunError::JobNotExpired
        );

        let escrow = job.escrow_lamports;
        transfer_lamports_from_program_owned(
            &ctx.accounts.job.to_account_info(),
            &ctx.accounts.client.to_account_info(),
            escrow,
        )?;

        unlock_all_assigned_stakes(
            job.required_lock_lamports,
            &job.assigned_workers,
            [
                &mut ctx.accounts.worker_stake_0,
                &mut ctx.accounts.worker_stake_1,
                &mut ctx.accounts.worker_stake_2,
            ],
        )?;

        let job = &mut ctx.accounts.job;
        job.escrow_lamports = 0;
        job.status = JobStatus::Cancelled as u8;

        Ok(())
    }

    pub fn slash_worker(ctx: Context<SlashWorker>) -> Result<()> {
        let job = &ctx.accounts.job;
        let stake = &mut ctx.accounts.worker_stake;
        let slash_amount = job.required_lock_lamports;
        let result = &ctx.accounts.job_result;

        require!(
            job.status == JobStatus::Finalized as u8,
            EdgerunError::InvalidJobState
        );
        require!(
            job.assigned_workers.contains(&stake.worker),
            EdgerunError::WorkerNotAssigned
        );
        require!(result.job_id == job.job_id, EdgerunError::InvalidJobResult);
        require!(result.worker == stake.worker, EdgerunError::WorkerMismatch);
        require!(
            result.output_hash != job.winning_output_hash,
            EdgerunError::WorkerNotSlashable
        );
        require!(slash_amount > 0, EdgerunError::InvalidAmount);

        stake.total_stake_lamports = stake
            .total_stake_lamports
            .checked_sub(slash_amount)
            .ok_or(EdgerunError::InsufficientStake)?;
        stake.locked_stake_lamports = stake
            .locked_stake_lamports
            .checked_sub(slash_amount)
            .ok_or(EdgerunError::MathOverflow)?;

        transfer_lamports_from_program_owned(
            &ctx.accounts.worker_stake.to_account_info(),
            &ctx.accounts.config.to_account_info(),
            slash_amount,
        )?;

        Ok(())
    }

    pub fn reach_quorum(ctx: Context<ReachQuorum>) -> Result<()> {
        let job = &mut ctx.accounts.job;
        require!(
            job.status == JobStatus::Assigned as u8,
            EdgerunError::InvalidJobState
        );
        require!(
            Clock::get()?.slot <= job.deadline_slot,
            EdgerunError::JobResultSubmissionExpired
        );

        let winning_output_hash = parse_quorum_winning_hash(job, &ctx.remaining_accounts)?
            .ok_or(error!(EdgerunError::QuorumNotMet))?;
        let now_slot = Clock::get()?.slot;
        job.winning_output_hash = winning_output_hash;
        job.quorum_reached_slot = now_slot;
        job.da_deadline_slot = now_slot
            .checked_add(ctx.accounts.config.da_window_slots)
            .ok_or(EdgerunError::MathOverflow)?;
        job.status = JobStatus::AwaitingDa as u8;
        Ok(())
    }

    pub fn declare_output(
        ctx: Context<DeclareOutput>,
        output_hash: [u8; 32],
        pointer: Vec<u8>,
    ) -> Result<()> {
        let job = &ctx.accounts.job;
        require!(
            job.status == JobStatus::AwaitingDa as u8,
            EdgerunError::InvalidJobState
        );
        require!(
            ctx.accounts.job_result.job_id == job.job_id,
            EdgerunError::InvalidJobResult
        );
        require!(
            ctx.accounts.job_result.worker == ctx.accounts.publisher.key(),
            EdgerunError::WorkerMismatch
        );
        require!(
            ctx.accounts.job_result.output_hash == job.winning_output_hash,
            EdgerunError::WorkerNotSlashable
        );
        require!(
            output_hash == job.winning_output_hash,
            EdgerunError::OutputHashMismatch
        );
        require!(pointer.len() <= 128, EdgerunError::PointerTooLarge);

        let mut pointer_fixed = [0_u8; 128];
        pointer_fixed[..pointer.len()].copy_from_slice(&pointer);
        let da = &mut ctx.accounts.output_availability;
        da.job_id = job.job_id;
        da.output_hash = output_hash;
        da.publisher = ctx.accounts.publisher.key();
        da.pointer = pointer_fixed;
        da.published_slot = Clock::get()?.slot;
        Ok(())
    }

    pub fn penalize_non_submitters(ctx: Context<PenalizeNonSubmitters>) -> Result<()> {
        let job = &ctx.accounts.job;
        let now_slot = Clock::get()?.slot;
        require!(
            job.status == JobStatus::Finalized as u8 || now_slot > job.deadline_slot,
            EdgerunError::InvalidJobState
        );

        let submitted_workers = collect_submitted_workers(job, &ctx.remaining_accounts)?;
        apply_non_response_slash(
            &mut ctx.accounts.worker_stake_0,
            &ctx.accounts.config,
            &ctx.accounts.job,
            &submitted_workers,
        )?;
        apply_non_response_slash(
            &mut ctx.accounts.worker_stake_1,
            &ctx.accounts.config,
            &ctx.accounts.job,
            &submitted_workers,
        )?;
        apply_non_response_slash(
            &mut ctx.accounts.worker_stake_2,
            &ctx.accounts.config,
            &ctx.accounts.job,
            &submitted_workers,
        )?;
        Ok(())
    }

    pub fn cancel_no_da(ctx: Context<CancelNoDa>) -> Result<()> {
        let now_slot = Clock::get()?.slot;
        let job = &mut ctx.accounts.job;
        require!(
            job.status == JobStatus::AwaitingDa as u8,
            EdgerunError::InvalidJobState
        );
        require!(now_slot > job.da_deadline_slot, EdgerunError::DaWindowActive);
        job.winning_output_hash = [0_u8; 32];
        job.quorum_reached_slot = 0;
        job.da_deadline_slot = 0;
        job.deadline_slot = now_slot
            .checked_add(ctx.accounts.config.challenge_window_slots)
            .ok_or(EdgerunError::MathOverflow)?;
        job.status = JobStatus::Assigned as u8;
        Ok(())
    }
}

struct ParsedFinalizeInputs {
    winning_output_hash: [u8; 32],
    winner_keys: Vec<Pubkey>,
    winner_accounts: Vec<Pubkey>,
}

fn parse_finalize_inputs(
    job: &Account<Job>,
    remaining_accounts: &[AccountInfo],
) -> Result<ParsedFinalizeInputs> {
    let mut seen_workers = HashSet::new();
    let mut by_output_hash: HashMap<[u8; 32], Vec<Pubkey>> = HashMap::new();
    let mut payout_candidates = HashSet::new();

    for account in remaining_accounts {
        if let Some(result) = parse_job_result_account(account)? {
            require!(result.job_id == job.job_id, EdgerunError::InvalidJobResult);
            require!(
                job.assigned_workers.contains(&result.worker),
                EdgerunError::WorkerNotAssigned
            );
            require!(
                seen_workers.insert(result.worker),
                EdgerunError::DuplicateJobResult
            );
            by_output_hash
                .entry(result.output_hash)
                .or_default()
                .push(result.worker);
        } else {
            if account.is_writable {
                payout_candidates.insert(*account.key);
            }
        }
    }

    let quorum_target = usize::from(job.quorum.max(1));
    let mut winning_output_hash = None;
    let mut winner_keys: Vec<Pubkey> = Vec::new();
    let mut winner_count = 0usize;
    let mut saw_tie = false;
    for (hash, workers) in by_output_hash {
        let worker_count = workers.len();
        if worker_count < quorum_target {
            continue;
        }
        if worker_count > winner_count {
            winning_output_hash = Some(hash);
            winner_keys = workers;
            winner_count = worker_count;
            saw_tie = false;
        } else if worker_count == winner_count {
            saw_tie = true;
        }
    }

    require!(!saw_tie, EdgerunError::QuorumNotMet);
    let winning_output_hash = winning_output_hash.ok_or(error!(EdgerunError::QuorumNotMet))?;

    let mut winner_accounts = Vec::with_capacity(winner_keys.len());
    for winner in &winner_keys {
        require!(
            payout_candidates.contains(winner),
            EdgerunError::MissingWinnerPayoutAccount
        );
        winner_accounts.push(*winner);
    }

    Ok(ParsedFinalizeInputs {
        winning_output_hash,
        winner_keys,
        winner_accounts,
    })
}

fn parse_job_result_account(account: &AccountInfo) -> Result<Option<JobResult>> {
    if account.owner != &crate::ID {
        return Ok(None);
    }

    let data = account
        .try_borrow_data()
        .map_err(|_| error!(EdgerunError::InvalidJobResult))?;
    if data.len() < 8 || &data[..8] != JobResult::DISCRIMINATOR {
        return Ok(None);
    }

    let mut bytes: &[u8] = &data;
    let parsed =
        JobResult::try_deserialize(&mut bytes).map_err(|_| error!(EdgerunError::InvalidJobResult))?;
    Ok(Some(parsed))
}

fn required_lock_for_job(escrow_lamports: u64, committee_size: u8) -> u64 {
    if committee_size == 0 {
        return 0;
    }

    // Formula from MVP spec: escrow * 3 / 2 / committee_size
    escrow_lamports
        .saturating_mul(3)
        .saturating_div(2)
        .saturating_div(u64::from(committee_size))
}

fn committee_tier_for_escrow(escrow_lamports: u64) -> (u8, u8) {
    const TIER0: u64 = 100_000_000;
    const TIER1: u64 = 1_000_000_000;
    const TIER2: u64 = 10_000_000_000;
    if escrow_lamports < TIER0 {
        (3, 2)
    } else if escrow_lamports < TIER1 {
        (5, 3)
    } else if escrow_lamports < TIER2 {
        (7, 5)
    } else {
        (9, 6)
    }
}

fn lock_worker_for_job(
    stake: &mut Account<WorkerStake>,
    worker_key: Pubkey,
    required_lock: u64,
) -> Result<()> {
    require!(stake.worker == worker_key, EdgerunError::WorkerMismatch);
    require!(
        stake.status == WorkerStatus::Active as u8,
        EdgerunError::WorkerNotActive
    );

    let available = stake
        .total_stake_lamports
        .checked_sub(stake.locked_stake_lamports)
        .ok_or(EdgerunError::MathOverflow)?;
    require!(available >= required_lock, EdgerunError::InsufficientStake);

    stake.locked_stake_lamports = stake
        .locked_stake_lamports
        .checked_add(required_lock)
        .ok_or(EdgerunError::MathOverflow)?;

    Ok(())
}

fn unlock_winner_stakes(
    required_lock_lamports: u64,
    winner_keys: &[Pubkey],
    worker_stakes: [&mut Account<WorkerStake>; 3],
) -> Result<()> {
    if required_lock_lamports == 0 {
        return Ok(());
    }

    for stake in worker_stakes {
        if winner_keys.contains(&stake.worker) {
            stake.locked_stake_lamports = stake
                .locked_stake_lamports
                .checked_sub(required_lock_lamports)
                .ok_or(EdgerunError::MathOverflow)?;
        }
    }

    Ok(())
}

fn unlock_all_assigned_stakes(
    required_lock_lamports: u64,
    assigned_workers: &[Pubkey; 9],
    worker_stakes: [&mut Account<WorkerStake>; 3],
) -> Result<()> {
    if required_lock_lamports == 0 {
        return Ok(());
    }

    for stake in worker_stakes {
        if assigned_workers.contains(&stake.worker) {
            stake.locked_stake_lamports = stake
                .locked_stake_lamports
                .checked_sub(required_lock_lamports)
                .ok_or(EdgerunError::MathOverflow)?;
        }
    }

    Ok(())
}

fn parse_quorum_winning_hash(
    job: &Account<Job>,
    remaining_accounts: &[AccountInfo],
) -> Result<Option<[u8; 32]>> {
    let mut seen_workers = HashSet::new();
    let mut by_output_hash: HashMap<[u8; 32], usize> = HashMap::new();
    for account in remaining_accounts {
        if let Some(result) = parse_job_result_account(account)? {
            require!(result.job_id == job.job_id, EdgerunError::InvalidJobResult);
            require!(
                job.assigned_workers.contains(&result.worker),
                EdgerunError::WorkerNotAssigned
            );
            require!(
                seen_workers.insert(result.worker),
                EdgerunError::DuplicateJobResult
            );
            *by_output_hash.entry(result.output_hash).or_insert(0) += 1;
        }
    }

    let quorum_target = usize::from(job.quorum.max(1));
    let mut best: Option<([u8; 32], usize)> = None;
    let mut saw_tie = false;
    for (hash, count) in by_output_hash {
        if count < quorum_target {
            continue;
        }
        match best {
            None => best = Some((hash, count)),
            Some((_, best_count)) if count > best_count => {
                best = Some((hash, count));
                saw_tie = false;
            }
            Some((_, best_count)) if count == best_count => {
                saw_tie = true;
            }
            _ => {}
        }
    }

    if saw_tie {
        return Ok(None);
    }
    Ok(best.map(|v| v.0))
}

fn collect_submitted_workers(job: &Account<Job>, remaining_accounts: &[AccountInfo]) -> Result<HashSet<Pubkey>> {
    let mut out = HashSet::new();
    for account in remaining_accounts {
        if let Some(result) = parse_job_result_account(account)? {
            if result.job_id == job.job_id && job.assigned_workers.contains(&result.worker) {
                out.insert(result.worker);
            }
        }
    }
    Ok(out)
}

fn apply_non_response_slash(
    stake: &mut Account<WorkerStake>,
    config: &Account<GlobalConfig>,
    job: &Account<Job>,
    submitted_workers: &HashSet<Pubkey>,
) -> Result<()> {
    if !job.assigned_workers.contains(&stake.worker) || submitted_workers.contains(&stake.worker) {
        return Ok(());
    }
    let available = stake
        .total_stake_lamports
        .checked_sub(stake.locked_stake_lamports)
        .ok_or(EdgerunError::MathOverflow)?;
    let slash_amount = available.min(config.non_response_slash_lamports);
    if slash_amount == 0 {
        return Ok(());
    }
    stake.total_stake_lamports = stake
        .total_stake_lamports
        .checked_sub(slash_amount)
        .ok_or(EdgerunError::InsufficientStake)?;
    transfer_lamports_from_program_owned(
        &stake.to_account_info(),
        &config.to_account_info(),
        slash_amount,
    )?;
    Ok(())
}

fn transfer_lamports_from_program_owned(
    from: &AccountInfo,
    to: &AccountInfo,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    let from_lamports = from.lamports();
    require!(
        from_lamports >= amount,
        EdgerunError::InsufficientVaultLamports
    );
    let to_lamports = to.lamports();

    **from.try_borrow_mut_lamports()? = from_lamports
        .checked_sub(amount)
        .ok_or(EdgerunError::MathOverflow)?;
    **to.try_borrow_mut_lamports()? = to_lamports
        .checked_add(amount)
        .ok_or(EdgerunError::MathOverflow)?;

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + GlobalConfig::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump, has_one = admin)]
    pub config: Account<'info, GlobalConfig>,
}

#[derive(Accounts)]
pub struct RegisterWorkerStake<'info> {
    #[account(mut)]
    pub worker: Signer<'info>,
    #[account(
        init,
        payer = worker,
        space = 8 + WorkerStake::INIT_SPACE,
        seeds = [b"worker_stake", worker.key().as_ref()],
        bump
    )]
    pub worker_stake: Account<'info, WorkerStake>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositStake<'info> {
    #[account(mut)]
    pub worker: Signer<'info>,
    #[account(mut, seeds = [b"worker_stake", worker.key().as_ref()], bump)]
    pub worker_stake: Account<'info, WorkerStake>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawStake<'info> {
    #[account(mut)]
    pub worker: Signer<'info>,
    #[account(mut, seeds = [b"worker_stake", worker.key().as_ref()], bump)]
    pub worker_stake: Account<'info, WorkerStake>,
}

#[derive(Accounts)]
#[instruction(job_id: [u8; 32])]
pub struct PostJob<'info> {
    #[account(mut)]
    pub client: Signer<'info>,
    #[account(seeds = [b"config"], bump)]
    pub config: Account<'info, GlobalConfig>,
    #[account(
        init,
        payer = client,
        space = 8 + Job::INIT_SPACE,
        seeds = [b"job", job_id.as_ref()],
        bump
    )]
    pub job: Account<'info, Job>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AssignWorkers<'info> {
    pub scheduler_authority: Signer<'info>,
    #[account(
        seeds = [b"config"],
        bump,
        has_one = scheduler_authority
    )]
    pub config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub job: Account<'info, Job>,
    #[account(mut)]
    pub worker_stake_0: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_1: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_2: Account<'info, WorkerStake>,
}

#[derive(Accounts)]
pub struct SetJobSeed<'info> {
    pub randomness_authority: Signer<'info>,
    #[account(
        seeds = [b"config"],
        bump,
        has_one = randomness_authority
    )]
    pub config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub job: Account<'info, Job>,
}

#[derive(Accounts)]
pub struct SubmitResult<'info> {
    #[account(mut)]
    pub worker: Signer<'info>,
    pub job: Account<'info, Job>,
    #[account(
        init,
        payer = worker,
        space = 8 + JobResult::INIT_SPACE,
        seeds = [b"job_result", job.job_id.as_ref(), worker.key().as_ref()],
        bump
    )]
    pub job_result: Account<'info, JobResult>,
    /// CHECK: verified by address constraint; parsed by sysvar instruction helpers.
    #[account(address = anchor_lang::solana_program::sysvar::instructions::id())]
    pub instructions_sysvar: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeJob<'info> {
    pub caller: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump)]
    pub config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub job: Account<'info, Job>,
    #[account(mut)]
    pub worker_stake_0: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_1: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_2: Account<'info, WorkerStake>,
}

#[derive(Accounts)]
pub struct CancelExpiredJob<'info> {
    pub caller: Signer<'info>,
    #[account(seeds = [b"config"], bump)]
    pub config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub job: Account<'info, Job>,
    #[account(mut, address = job.client)]
    pub client: SystemAccount<'info>,
    #[account(mut)]
    pub worker_stake_0: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_1: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_2: Account<'info, WorkerStake>,
}

#[derive(Accounts)]
pub struct SlashWorker<'info> {
    pub caller: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump)]
    pub config: Account<'info, GlobalConfig>,
    pub job: Account<'info, Job>,
    #[account(mut)]
    pub worker_stake: Account<'info, WorkerStake>,
    pub job_result: Account<'info, JobResult>,
}

#[derive(Accounts)]
pub struct ReachQuorum<'info> {
    pub caller: Signer<'info>,
    #[account(seeds = [b"config"], bump)]
    pub config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub job: Account<'info, Job>,
}

#[derive(Accounts)]
pub struct DeclareOutput<'info> {
    #[account(mut)]
    pub publisher: Signer<'info>,
    pub job: Account<'info, Job>,
    #[account(
        seeds = [b"job_result", job.job_id.as_ref(), publisher.key().as_ref()],
        bump
    )]
    pub job_result: Account<'info, JobResult>,
    #[account(
        init,
        payer = publisher,
        space = 8 + OutputAvailability::INIT_SPACE,
        seeds = [b"output", job.job_id.as_ref()],
        bump
    )]
    pub output_availability: Account<'info, OutputAvailability>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PenalizeNonSubmitters<'info> {
    pub caller: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump)]
    pub config: Account<'info, GlobalConfig>,
    pub job: Account<'info, Job>,
    #[account(mut)]
    pub worker_stake_0: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_1: Account<'info, WorkerStake>,
    #[account(mut)]
    pub worker_stake_2: Account<'info, WorkerStake>,
}

#[derive(Accounts)]
pub struct CancelNoDa<'info> {
    pub caller: Signer<'info>,
    #[account(seeds = [b"config"], bump)]
    pub config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub job: Account<'info, Job>,
}

#[account]
#[derive(InitSpace)]
pub struct GlobalConfig {
    pub admin: Pubkey,
    pub scheduler_authority: Pubkey,
    pub randomness_authority: Pubkey,
    pub min_worker_stake_lamports: u64,
    pub protocol_fee_bps: u16,
    pub committee_size: u8,
    pub quorum: u8,
    pub challenge_window_slots: u64,
    pub da_window_slots: u64,
    pub non_response_slash_lamports: u64,
    pub committee_tiering_enabled: bool,
    pub max_committee_size: u8,
    pub max_memory_bytes: u32,
    pub max_instructions: u64,
    pub allowed_runtime_root: [u8; 32],
    pub paused: bool,
}

#[account]
#[derive(InitSpace)]
pub struct WorkerStake {
    pub worker: Pubkey,
    pub total_stake_lamports: u64,
    pub locked_stake_lamports: u64,
    pub status: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Job {
    pub job_id: [u8; 32],
    pub client: Pubkey,
    pub escrow_lamports: u64,
    pub bundle_hash: [u8; 32],
    pub runtime_id: [u8; 32],
    pub max_memory_bytes: u32,
    pub max_instructions: u64,
    pub committee_size: u8,
    pub quorum: u8,
    pub created_slot: u64,
    pub deadline_slot: u64,
    pub assigned_workers: [Pubkey; 9],
    pub assigned_count: u8,
    pub required_lock_lamports: u64,
    pub seed: [u8; 32],
    pub seed_set: bool,
    pub winning_output_hash: [u8; 32],
    pub quorum_reached_slot: u64,
    pub da_deadline_slot: u64,
    pub status: u8,
}

#[account]
#[derive(InitSpace)]
pub struct JobResult {
    pub job_id: [u8; 32],
    pub worker: Pubkey,
    pub output_hash: [u8; 32],
    pub attestation_sig: [u8; 64],
    pub submitted_slot: u64,
}

#[account]
#[derive(InitSpace)]
pub struct OutputAvailability {
    pub job_id: [u8; 32],
    pub output_hash: [u8; 32],
    pub publisher: Pubkey,
    pub pointer: [u8; 128],
    pub published_slot: u64,
}

#[repr(u8)]
pub enum WorkerStatus {
    Active = 0,
    Jailed = 1,
}

#[repr(u8)]
pub enum JobStatus {
    Posted = 0,
    Assigned = 1,
    Finalized = 2,
    Cancelled = 3,
    Slashed = 4,
    Seeded = 5,
    QuorumReached = 6,
    AwaitingDa = 7,
}

#[error_code]
pub enum EdgerunError {
    #[msg("program is paused")]
    Paused,
    #[msg("requested limit exceeds configured cap")]
    LimitExceeded,
    #[msg("worker not active")]
    WorkerNotActive,
    #[msg("worker not assigned")]
    WorkerNotAssigned,
    #[msg("duplicate assigned worker")]
    DuplicateAssignedWorker,
    #[msg("worker mismatch")]
    WorkerMismatch,
    #[msg("insufficient stake")]
    InsufficientStake,
    #[msg("stake is locked")]
    StakeLocked,
    #[msg("invalid job state")]
    InvalidJobState,
    #[msg("quorum not met")]
    QuorumNotMet,
    #[msg("job is not expired")]
    JobNotExpired,
    #[msg("math overflow")]
    MathOverflow,
    #[msg("invalid amount")]
    InvalidAmount,
    #[msg("insufficient vault lamports")]
    InsufficientVaultLamports,
    #[msg("rent exemption violation")]
    RentExemptionViolation,
    #[msg("invalid winner account list")]
    InvalidWinnerAccounts,
    #[msg("winner is not an assigned worker")]
    WinnerNotAssigned,
    #[msg("invalid bundle hash")]
    InvalidBundleHash,
    #[msg("invalid runtime id")]
    InvalidRuntimeId,
    #[msg("runtime is not allowed by config")]
    RuntimeNotAllowed,
    #[msg("missing ed25519 attestation pre-instruction")]
    MissingEd25519Instruction,
    #[msg("invalid ed25519 attestation instruction")]
    InvalidEd25519Instruction,
    #[msg("attestation message mismatch")]
    AttestationMessageMismatch,
    #[msg("attestation signature mismatch")]
    AttestationSignatureMismatch,
    #[msg("attestation signer mismatch")]
    AttestationSignerMismatch,
    #[msg("invalid job result")]
    InvalidJobResult,
    #[msg("duplicate job result for worker")]
    DuplicateJobResult,
    #[msg("missing winner payout account")]
    MissingWinnerPayoutAccount,
    #[msg("winner payout account must be writable")]
    PayoutAccountNotWritable,
    #[msg("worker result is not slashable")]
    WorkerNotSlashable,
    #[msg("job result submission is past deadline")]
    JobResultSubmissionExpired,
    #[msg("caller is not authorized to cancel this job")]
    UnauthorizedCancelCaller,
    #[msg("job seed has not been set")]
    SeedNotSet,
    #[msg("job seed already set")]
    SeedAlreadySet,
    #[msg("invalid seed")]
    InvalidSeed,
    #[msg("output hash mismatch")]
    OutputHashMismatch,
    #[msg("output pointer too large")]
    PointerTooLarge,
    #[msg("DA window still active")]
    DaWindowActive,
}

fn verify_worker_attestation(
    instructions_sysvar: &UncheckedAccount,
    worker: &Pubkey,
    job_id: &[u8; 32],
    bundle_hash: &[u8; 32],
    output_hash: &[u8; 32],
    runtime_id: &[u8; 32],
    attestation_sig: &[u8; 64],
) -> Result<()> {
    let current_index = load_current_index_checked(&instructions_sysvar.to_account_info())
        .map_err(|_| error!(EdgerunError::InvalidEd25519Instruction))?;
    require!(current_index > 0, EdgerunError::MissingEd25519Instruction);

    let verify_ix = load_instruction_at_checked(
        usize::from(current_index - 1),
        &instructions_sysvar.to_account_info(),
    )
    .map_err(|_| error!(EdgerunError::MissingEd25519Instruction))?;

    require!(
        verify_ix.program_id == ed25519_program::id(),
        EdgerunError::MissingEd25519Instruction
    );
    require!(
        verify_ix.accounts.is_empty(),
        EdgerunError::InvalidEd25519Instruction
    );

    let data = verify_ix.data;
    require!(data.len() >= 16, EdgerunError::InvalidEd25519Instruction);
    require!(data[0] == 1, EdgerunError::InvalidEd25519Instruction);

    let signature_offset = read_u16_le(&data, 2)?;
    let signature_instruction_index = read_u16_le(&data, 4)?;
    let public_key_offset = read_u16_le(&data, 6)?;
    let public_key_instruction_index = read_u16_le(&data, 8)?;
    let message_data_offset = read_u16_le(&data, 10)?;
    let message_data_size = read_u16_le(&data, 12)?;
    let message_instruction_index = read_u16_le(&data, 14)?;

    require!(
        signature_instruction_index == u16::MAX
            && public_key_instruction_index == u16::MAX
            && message_instruction_index == u16::MAX,
        EdgerunError::InvalidEd25519Instruction
    );

    let signature_range = range_checked(signature_offset, 64, data.len())?;
    let pubkey_range = range_checked(public_key_offset, 32, data.len())?;
    let message_range = range_checked(message_data_offset, message_data_size, data.len())?;

    require!(
        data[signature_range] == *attestation_sig,
        EdgerunError::AttestationSignatureMismatch
    );
    require!(
        data[pubkey_range] == worker.to_bytes(),
        EdgerunError::AttestationSignerMismatch
    );

    let expected_message = build_result_digest(job_id, bundle_hash, output_hash, runtime_id);
    require!(
        data[message_range] == expected_message,
        EdgerunError::AttestationMessageMismatch
    );

    Ok(())
}

fn build_result_digest(
    job_id: &[u8; 32],
    bundle_hash: &[u8; 32],
    output_hash: &[u8; 32],
    runtime_id: &[u8; 32],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(job_id);
    hasher.update(bundle_hash);
    hasher.update(output_hash);
    hasher.update(runtime_id);
    *hasher.finalize().as_bytes()
}

fn verify_runtime_membership(
    allowed_runtime_root: &[u8; 32],
    runtime_id: &[u8; 32],
    proof: &[[u8; 32]],
) -> bool {
    if proof.len() > 32 {
        return false;
    }
    let mut acc = *runtime_id;
    for sibling in proof {
        acc = merkle_parent_sorted(&acc, sibling);
    }
    acc == *allowed_runtime_root
}

fn merkle_parent_sorted(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    if a <= b {
        hasher.update(a);
        hasher.update(b);
    } else {
        hasher.update(b);
        hasher.update(a);
    }
    *hasher.finalize().as_bytes()
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or(error!(EdgerunError::InvalidEd25519Instruction))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn range_checked(offset: u16, len: u16, total_len: usize) -> Result<std::ops::Range<usize>> {
    let start = usize::from(offset);
    let end = start
        .checked_add(usize::from(len))
        .ok_or(error!(EdgerunError::InvalidEd25519Instruction))?;
    require!(end <= total_len, EdgerunError::InvalidEd25519Instruction);
    Ok(start..end)
}
