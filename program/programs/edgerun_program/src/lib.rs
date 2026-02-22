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

declare_id!("A2ac8yDnTXKfZCHWqcJVYFfR2jv65kezW95XTgrrdbtG");

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
        let (committee_size, quorum) = derive_job_committee_and_quorum(config, escrow_lamports);
        job.job_id = job_id;
        job.client = ctx.accounts.client.key();
        job.escrow_lamports = escrow_lamports;
        // Boundary contract: this must be compute_bundle_hash(canonical_bundle_payload_bytes).
        job.bundle_hash = bundle_hash;
        job.runtime_id = runtime_id;
        job.max_memory_bytes = max_memory_bytes;
        job.max_instructions = max_instructions;
        job.committee_size = committee_size;
        job.quorum = quorum;
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

    pub fn assign_workers<'info>(
        ctx: Context<'_, '_, 'info, 'info, AssignWorkers<'info>>,
        workers: Vec<Pubkey>,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        let job = &mut ctx.accounts.job;
        validate_assign_workers_request(job, config, &workers, ctx.remaining_accounts.len())?;

        let required_lock = required_lock_for_job(job.escrow_lamports, job.committee_size)
            .max(config.min_worker_stake_lamports);
        for (worker_key, account_info) in workers.iter().zip(ctx.remaining_accounts.iter()) {
            require!(
                account_info.is_writable,
                EdgerunError::InvalidWorkerStakeAccountList
            );
            with_worker_stake_mut(account_info, |worker_stake| {
                lock_worker_for_job(worker_stake, *worker_key, required_lock)
            })?;
        }

        job.assigned_workers = [Pubkey::default(); 9];
        for (idx, worker) in workers.iter().enumerate() {
            job.assigned_workers[idx] = *worker;
        }
        job.assigned_count = workers.len() as u8;
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
        validate_submit_result_request(job, &ctx.accounts.worker.key(), Clock::get()?.slot)?;
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

    pub fn finalize_job<'info>(
        ctx: Context<'_, '_, 'info, 'info, FinalizeJob<'info>>,
    ) -> Result<()> {
        let job = &ctx.accounts.job;
        require!(
            job.status == JobStatus::AwaitingDa as u8,
            EdgerunError::InvalidJobState
        );
        validate_output_availability_for_finalize(
            &job.job_id,
            &job.winning_output_hash,
            &ctx.accounts.output_availability,
        )?;
        let finalize_inputs = parse_finalize_inputs(job, &ctx.remaining_accounts)?;
        let winner_count = finalize_inputs.winner_keys.len();

        let assigned_workers = assigned_worker_set(job);
        let winner_set: HashSet<Pubkey> = finalize_inputs.winner_keys.iter().copied().collect();
        let mut unlocked = HashSet::new();
        if job.required_lock_lamports > 0 {
            for account_info in ctx.remaining_accounts.iter() {
                if let Some(stored_stake) = parse_worker_stake_account(account_info)? {
                    require!(
                        account_info.is_writable,
                        EdgerunError::InvalidWorkerStakeAccountList
                    );
                    if !assigned_workers.contains(&stored_stake.worker)
                        || !winner_set.contains(&stored_stake.worker)
                    {
                        continue;
                    }
                    with_worker_stake_mut(account_info, |stake| {
                        stake.locked_stake_lamports = stake
                            .locked_stake_lamports
                            .checked_sub(job.required_lock_lamports)
                            .ok_or(EdgerunError::MathOverflow)?;
                        Ok(())
                    })?;
                    unlocked.insert(stored_stake.worker);
                }
            }
            require!(
                unlocked.len() == winner_set.len(),
                EdgerunError::InvalidWorkerStakeAccountList
            );
        }

        let escrow_lamports = job.escrow_lamports;
        let (protocol_fee, payout_each, payout_remainder) = compute_finalize_payouts(
            escrow_lamports,
            ctx.accounts.config.protocol_fee_bps,
            winner_count,
        )?;

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

    pub fn cancel_expired_job<'info>(
        ctx: Context<'_, '_, 'info, 'info, CancelExpiredJob<'info>>,
    ) -> Result<()> {
        let job = &ctx.accounts.job;
        let current_slot = Clock::get()?.slot;
        validate_cancel_expired_request(
            job,
            &ctx.accounts.config,
            ctx.accounts.caller.key(),
            current_slot,
        )?;

        let escrow = job.escrow_lamports;
        transfer_lamports_from_program_owned(
            &ctx.accounts.job.to_account_info(),
            &ctx.accounts.client.to_account_info(),
            escrow,
        )?;

        if job.required_lock_lamports > 0 {
            let assigned_workers = assigned_worker_set(job);
            let mut unlocked = HashSet::new();
            for account_info in ctx.remaining_accounts.iter() {
                if let Some(stored_stake) = parse_worker_stake_account(account_info)? {
                    require!(
                        account_info.is_writable,
                        EdgerunError::InvalidWorkerStakeAccountList
                    );
                    if !assigned_workers.contains(&stored_stake.worker) {
                        continue;
                    }
                    with_worker_stake_mut(account_info, |stake| {
                        stake.locked_stake_lamports = stake
                            .locked_stake_lamports
                            .checked_sub(job.required_lock_lamports)
                            .ok_or(EdgerunError::MathOverflow)?;
                        Ok(())
                    })?;
                    unlocked.insert(stored_stake.worker);
                }
            }
            require!(
                unlocked.len() == assigned_workers.len(),
                EdgerunError::InvalidWorkerStakeAccountList
            );
        }

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

    pub fn penalize_non_submitters<'info>(
        ctx: Context<'_, '_, 'info, 'info, PenalizeNonSubmitters<'info>>,
    ) -> Result<()> {
        let job = &ctx.accounts.job;
        let now_slot = Clock::get()?.slot;
        require!(
            job.status == JobStatus::Finalized as u8 || now_slot > job.deadline_slot,
            EdgerunError::InvalidJobState
        );

        let submitted_workers = collect_submitted_workers(job, &ctx.remaining_accounts)?;
        let assigned_workers = assigned_worker_set(job);
        let mut processed = HashSet::new();
        for account_info in ctx.remaining_accounts.iter() {
            if let Some(stored_stake) = parse_worker_stake_account(account_info)? {
                require!(
                    account_info.is_writable,
                    EdgerunError::InvalidWorkerStakeAccountList
                );
                if !assigned_workers.contains(&stored_stake.worker) {
                    continue;
                }
                apply_non_response_slash(
                    account_info,
                    &ctx.accounts.config,
                    &ctx.accounts.job,
                    &submitted_workers,
                )?;
                processed.insert(stored_stake.worker);
            }
        }
        require!(
            processed.len() == assigned_workers.len(),
            EdgerunError::InvalidWorkerStakeAccountList
        );
        Ok(())
    }

    pub fn cancel_no_da(ctx: Context<CancelNoDa>) -> Result<()> {
        let now_slot = Clock::get()?.slot;
        let job = &mut ctx.accounts.job;
        require!(
            job.status == JobStatus::AwaitingDa as u8,
            EdgerunError::InvalidJobState
        );
        require!(
            now_slot > job.da_deadline_slot,
            EdgerunError::DaWindowActive
        );
        reset_job_after_no_da(job, now_slot, ctx.accounts.config.challenge_window_slots)
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
    let mut results = Vec::new();
    let mut payout_candidates = HashSet::new();

    for account in remaining_accounts {
        if let Some(result) = parse_job_result_account(account)? {
            results.push(result);
        } else {
            if account.is_writable {
                payout_candidates.insert(*account.key);
            }
        }
    }
    finalize_inputs_from_results(job, &results, &payout_candidates)
}

fn finalize_inputs_from_results(
    job: &Job,
    results: &[JobResult],
    payout_candidates: &HashSet<Pubkey>,
) -> Result<ParsedFinalizeInputs> {
    let mut seen_workers = HashSet::new();
    let mut by_output_hash: HashMap<[u8; 32], Vec<Pubkey>> = HashMap::new();
    for result in results {
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
    let parsed = JobResult::try_deserialize(&mut bytes)
        .map_err(|_| error!(EdgerunError::InvalidJobResult))?;
    Ok(Some(parsed))
}

fn parse_worker_stake_account(account: &AccountInfo) -> Result<Option<WorkerStake>> {
    if account.owner != &crate::ID {
        return Ok(None);
    }

    let data = account
        .try_borrow_data()
        .map_err(|_| error!(EdgerunError::InvalidWorkerStakeAccount))?;
    if data.len() < 8 || &data[..8] != WorkerStake::DISCRIMINATOR {
        return Ok(None);
    }

    let mut bytes: &[u8] = &data;
    let parsed = WorkerStake::try_deserialize(&mut bytes)
        .map_err(|_| error!(EdgerunError::InvalidWorkerStakeAccount))?;
    Ok(Some(parsed))
}

fn with_worker_stake_mut<F>(account_info: &AccountInfo, mutator: F) -> Result<()>
where
    F: FnOnce(&mut WorkerStake) -> Result<()>,
{
    require!(
        account_info.owner == &crate::ID,
        EdgerunError::InvalidWorkerStakeAccount
    );
    require!(
        account_info.is_writable,
        EdgerunError::InvalidWorkerStakeAccountList
    );

    let mut data = account_info
        .try_borrow_mut_data()
        .map_err(|_| error!(EdgerunError::InvalidWorkerStakeAccount))?;
    if data.len() < 8 || &data[..8] != WorkerStake::DISCRIMINATOR {
        return Err(error!(EdgerunError::InvalidWorkerStakeAccount));
    }

    let mut bytes: &[u8] = &data;
    let mut stake = WorkerStake::try_deserialize(&mut bytes)
        .map_err(|_| error!(EdgerunError::InvalidWorkerStakeAccount))?;
    mutator(&mut stake)?;

    let mut out: &mut [u8] = &mut data;
    stake
        .try_serialize(&mut out)
        .map_err(|_| error!(EdgerunError::InvalidWorkerStakeAccount))?;
    Ok(())
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

fn derive_job_committee_and_quorum(config: &GlobalConfig, escrow_lamports: u64) -> (u8, u8) {
    let (tier_committee_size, tier_quorum) = committee_tier_for_escrow(escrow_lamports);
    let committee_size = if config.committee_tiering_enabled {
        tier_committee_size.min(config.max_committee_size.max(1))
    } else {
        config.committee_size.max(1)
    };
    let quorum = if config.committee_tiering_enabled {
        tier_quorum.min(committee_size)
    } else {
        config.quorum.min(committee_size).max(1)
    };
    (committee_size, quorum)
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
    stake: &mut WorkerStake,
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

fn validate_assign_workers_request(
    job: &Job,
    config: &GlobalConfig,
    workers: &[Pubkey],
    remaining_accounts_len: usize,
) -> Result<()> {
    require!(
        job.status == JobStatus::Posted as u8 || job.status == JobStatus::Seeded as u8,
        EdgerunError::InvalidJobState
    );
    if config.committee_tiering_enabled {
        require!(job.seed_set, EdgerunError::SeedNotSet);
    }
    let expected_count = usize::from(job.committee_size.max(1));
    require!(
        workers.len() == expected_count && workers.len() <= 9,
        EdgerunError::InvalidAssignedWorkerCount
    );
    require!(
        remaining_accounts_len == workers.len(),
        EdgerunError::InvalidWorkerStakeAccountList
    );
    let unique_workers: HashSet<Pubkey> = workers.iter().copied().collect();
    require!(
        unique_workers.len() == workers.len(),
        EdgerunError::DuplicateAssignedWorker
    );
    Ok(())
}

fn validate_submit_result_request(job: &Job, worker: &Pubkey, now_slot: u64) -> Result<()> {
    require!(
        job.status == JobStatus::Assigned as u8,
        EdgerunError::InvalidJobState
    );
    require!(
        job.assigned_workers.contains(worker),
        EdgerunError::WorkerNotAssigned
    );
    require!(
        now_slot <= job.deadline_slot,
        EdgerunError::JobResultSubmissionExpired
    );
    Ok(())
}

fn validate_cancel_expired_request(
    job: &Job,
    config: &GlobalConfig,
    caller: Pubkey,
    current_slot: u64,
) -> Result<()> {
    require!(
        caller == job.client || caller == config.scheduler_authority,
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
    Ok(())
}

fn assigned_worker_set(job: &Account<Job>) -> HashSet<Pubkey> {
    let assigned_count = usize::from(job.assigned_count.min(9));
    job.assigned_workers
        .iter()
        .take(assigned_count)
        .copied()
        .filter(|k| *k != Pubkey::default())
        .collect()
}

fn parse_quorum_winning_hash(
    job: &Account<Job>,
    remaining_accounts: &[AccountInfo],
) -> Result<Option<[u8; 32]>> {
    let mut parsed = Vec::new();
    for account in remaining_accounts {
        if let Some(result) = parse_job_result_account(account)? {
            parsed.push(result);
        }
    }
    quorum_winning_hash_from_results(job, &parsed)
}

fn quorum_winning_hash_from_results(job: &Job, results: &[JobResult]) -> Result<Option<[u8; 32]>> {
    let mut seen_workers = HashSet::new();
    let mut by_output_hash: HashMap<[u8; 32], usize> = HashMap::new();
    for result in results {
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

fn collect_submitted_workers(
    job: &Account<Job>,
    remaining_accounts: &[AccountInfo],
) -> Result<HashSet<Pubkey>> {
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

fn validate_output_availability_for_finalize(
    job_id: &[u8; 32],
    winning_output_hash: &[u8; 32],
    da: &Account<OutputAvailability>,
) -> Result<()> {
    require!(
        output_availability_matches(job_id, winning_output_hash, &da.job_id, &da.output_hash),
        EdgerunError::OutputHashMismatch
    );
    Ok(())
}

fn compute_finalize_payouts(
    escrow_lamports: u64,
    protocol_fee_bps: u16,
    winner_count: usize,
) -> Result<(u64, u64, u64)> {
    require!(winner_count > 0, EdgerunError::InvalidWinnerAccounts);
    let winner_count_u64 = winner_count as u64;
    let protocol_fee = escrow_lamports
        .checked_mul(u64::from(protocol_fee_bps))
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
    Ok((protocol_fee, payout_each, payout_remainder))
}

fn reset_job_after_no_da(job: &mut Job, now_slot: u64, challenge_window_slots: u64) -> Result<()> {
    job.winning_output_hash = [0_u8; 32];
    job.quorum_reached_slot = 0;
    job.da_deadline_slot = 0;
    job.deadline_slot = now_slot
        .checked_add(challenge_window_slots)
        .ok_or(EdgerunError::MathOverflow)?;
    job.status = JobStatus::Assigned as u8;
    Ok(())
}

fn output_availability_matches(
    job_id: &[u8; 32],
    winning_output_hash: &[u8; 32],
    da_job_id: &[u8; 32],
    da_output_hash: &[u8; 32],
) -> bool {
    da_job_id == job_id && da_output_hash == winning_output_hash && *da_output_hash != [0_u8; 32]
}

fn apply_non_response_slash(
    stake_info: &AccountInfo,
    config: &Account<GlobalConfig>,
    job: &Account<Job>,
    submitted_workers: &HashSet<Pubkey>,
) -> Result<()> {
    with_worker_stake_mut(stake_info, |stake| {
        if !job.assigned_workers.contains(&stake.worker)
            || submitted_workers.contains(&stake.worker)
        {
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
        transfer_lamports_from_program_owned(stake_info, &config.to_account_info(), slash_amount)?;
        Ok(())
    })
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
    #[account(seeds = [b"output", job.job_id.as_ref()], bump)]
    pub output_availability: Account<'info, OutputAvailability>,
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
    #[msg("invalid assigned worker count for job committee size")]
    InvalidAssignedWorkerCount,
    #[msg("invalid worker stake account")]
    InvalidWorkerStakeAccount,
    #[msg("invalid worker stake account list")]
    InvalidWorkerStakeAccountList,
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

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::pubkey::Pubkey;

    #[test]
    fn committee_tier_boundaries_match_phase2_table() {
        assert_eq!(committee_tier_for_escrow(99_999_999), (3, 2));
        assert_eq!(committee_tier_for_escrow(100_000_000), (5, 3));
        assert_eq!(committee_tier_for_escrow(999_999_999), (5, 3));
        assert_eq!(committee_tier_for_escrow(1_000_000_000), (7, 5));
        assert_eq!(committee_tier_for_escrow(9_999_999_999), (7, 5));
        assert_eq!(committee_tier_for_escrow(10_000_000_000), (9, 6));
    }

    #[test]
    fn output_availability_must_match_job_and_winner_hash() {
        let job_id = [1_u8; 32];
        let winning = [2_u8; 32];
        let other_job = [3_u8; 32];
        let other_hash = [4_u8; 32];

        assert!(output_availability_matches(
            &job_id, &winning, &job_id, &winning
        ));
        assert!(!output_availability_matches(
            &job_id, &winning, &other_job, &winning
        ));
        assert!(!output_availability_matches(
            &job_id,
            &winning,
            &job_id,
            &other_hash
        ));
        assert!(!output_availability_matches(
            &job_id,
            &winning,
            &job_id,
            &[0_u8; 32]
        ));
    }

    #[test]
    fn required_lock_formula_handles_zero_and_large_values() {
        assert_eq!(required_lock_for_job(500_000_000, 0), 0);
        assert_eq!(required_lock_for_job(600_000_000, 3), 300_000_000);
        assert_eq!(required_lock_for_job(900_000_000, 5), 270_000_000);
        assert_eq!(required_lock_for_job(u64::MAX, 3), u64::MAX / 6);
    }

    #[test]
    fn lock_worker_for_job_validates_status_owner_and_capacity() {
        let worker = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let mut stake = WorkerStake {
            worker,
            total_stake_lamports: 1_000_000,
            locked_stake_lamports: 250_000,
            status: WorkerStatus::Active as u8,
        };

        assert!(lock_worker_for_job(&mut stake, worker, 500_000).is_ok());
        assert_eq!(stake.locked_stake_lamports, 750_000);
        assert!(lock_worker_for_job(&mut stake, worker, 300_000).is_err());

        let mut jailed = WorkerStake {
            worker,
            total_stake_lamports: 1_000_000,
            locked_stake_lamports: 0,
            status: WorkerStatus::Jailed as u8,
        };
        assert!(lock_worker_for_job(&mut jailed, worker, 1).is_err());

        let mut wrong_owner = WorkerStake {
            worker,
            total_stake_lamports: 1_000_000,
            locked_stake_lamports: 0,
            status: WorkerStatus::Active as u8,
        };
        assert!(lock_worker_for_job(&mut wrong_owner, other, 1).is_err());
    }

    #[test]
    fn runtime_membership_verification_accepts_valid_proof_and_rejects_invalid() {
        let leaf = [7_u8; 32];
        let sibling = [9_u8; 32];
        let root = merkle_parent_sorted(&leaf, &sibling);

        assert!(verify_runtime_membership(&root, &leaf, &[sibling]));

        let wrong_sibling = [8_u8; 32];
        assert!(!verify_runtime_membership(&root, &leaf, &[wrong_sibling]));

        let too_many = vec![[1_u8; 32]; 33];
        assert!(!verify_runtime_membership(&root, &leaf, &too_many));
    }

    #[test]
    fn merkle_parent_sorted_is_order_independent() {
        let a = [1_u8; 32];
        let b = [2_u8; 32];
        assert_eq!(merkle_parent_sorted(&a, &b), merkle_parent_sorted(&b, &a));
    }

    #[test]
    fn result_digest_is_deterministic_and_input_sensitive() {
        let job_id = [1_u8; 32];
        let bundle_hash = [2_u8; 32];
        let output_hash = [3_u8; 32];
        let runtime_id = [4_u8; 32];

        let digest_a = build_result_digest(&job_id, &bundle_hash, &output_hash, &runtime_id);
        let digest_b = build_result_digest(&job_id, &bundle_hash, &output_hash, &runtime_id);
        assert_eq!(digest_a, digest_b);

        let changed = build_result_digest(&job_id, &bundle_hash, &[5_u8; 32], &runtime_id);
        assert_ne!(digest_a, changed);
    }

    #[test]
    fn parser_helpers_validate_bounds() {
        let bytes = [0x34_u8, 0x12_u8, 0xff_u8];
        assert_eq!(read_u16_le(&bytes, 0).unwrap(), 0x1234);
        assert!(read_u16_le(&bytes, 2).is_err());

        let ok = range_checked(2, 3, 8).unwrap();
        assert_eq!(ok, 2..5);
        assert!(range_checked(7, 2, 8).is_err());
        assert!(range_checked(u16::MAX, u16::MAX, 8).is_err());
    }

    fn test_job(assigned: &[Pubkey], quorum: u8, job_id: [u8; 32]) -> Job {
        let mut assigned_workers = [Pubkey::default(); 9];
        for (i, worker) in assigned.iter().enumerate() {
            assigned_workers[i] = *worker;
        }
        Job {
            job_id,
            client: Pubkey::default(),
            escrow_lamports: 0,
            bundle_hash: [0_u8; 32],
            runtime_id: [0_u8; 32],
            max_memory_bytes: 0,
            max_instructions: 0,
            committee_size: assigned.len() as u8,
            quorum,
            created_slot: 0,
            deadline_slot: 0,
            assigned_workers,
            assigned_count: assigned.len() as u8,
            required_lock_lamports: 0,
            seed: [0_u8; 32],
            seed_set: false,
            winning_output_hash: [0_u8; 32],
            quorum_reached_slot: 0,
            da_deadline_slot: 0,
            status: 0,
        }
    }

    fn test_result(job_id: [u8; 32], worker: Pubkey, output_hash: [u8; 32]) -> JobResult {
        JobResult {
            job_id,
            worker,
            output_hash,
            attestation_sig: [0_u8; 64],
            submitted_slot: 0,
        }
    }

    fn test_config() -> GlobalConfig {
        GlobalConfig {
            admin: Pubkey::default(),
            scheduler_authority: Pubkey::default(),
            randomness_authority: Pubkey::default(),
            min_worker_stake_lamports: 0,
            protocol_fee_bps: 100,
            committee_size: 3,
            quorum: 2,
            challenge_window_slots: 100,
            da_window_slots: 50,
            non_response_slash_lamports: 0,
            committee_tiering_enabled: false,
            max_committee_size: 9,
            max_memory_bytes: 0,
            max_instructions: 0,
            allowed_runtime_root: [0_u8; 32],
            paused: false,
        }
    }

    #[test]
    fn quorum_winner_selects_majority_hash() {
        let job_id = [1_u8; 32];
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let w2 = Pubkey::new_unique();
        let job = test_job(&[w0, w1, w2], 2, job_id);
        let h1 = [11_u8; 32];
        let h2 = [22_u8; 32];
        let results = vec![
            test_result(job_id, w0, h1),
            test_result(job_id, w1, h1),
            test_result(job_id, w2, h2),
        ];
        let winner = quorum_winning_hash_from_results(&job, &results).unwrap();
        assert_eq!(winner, Some(h1));
    }

    #[test]
    fn quorum_winner_returns_none_on_tie() {
        let job_id = [2_u8; 32];
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let w2 = Pubkey::new_unique();
        let w3 = Pubkey::new_unique();
        let job = test_job(&[w0, w1, w2, w3], 2, job_id);
        let h1 = [1_u8; 32];
        let h2 = [2_u8; 32];
        let results = vec![
            test_result(job_id, w0, h1),
            test_result(job_id, w1, h1),
            test_result(job_id, w2, h2),
            test_result(job_id, w3, h2),
        ];
        let winner = quorum_winning_hash_from_results(&job, &results).unwrap();
        assert_eq!(winner, None);
    }

    #[test]
    fn quorum_winner_errors_on_duplicate_unassigned_or_wrong_job() {
        let job_id = [3_u8; 32];
        let other_job = [4_u8; 32];
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let outsider = Pubkey::new_unique();
        let job = test_job(&[w0, w1], 2, job_id);
        let h1 = [9_u8; 32];

        let duplicate = vec![test_result(job_id, w0, h1), test_result(job_id, w0, h1)];
        assert!(quorum_winning_hash_from_results(&job, &duplicate).is_err());

        let unassigned = vec![test_result(job_id, outsider, h1)];
        assert!(quorum_winning_hash_from_results(&job, &unassigned).is_err());

        let wrong_job = vec![test_result(other_job, w0, h1)];
        assert!(quorum_winning_hash_from_results(&job, &wrong_job).is_err());
    }

    #[test]
    fn finalize_inputs_selects_winners_and_requires_payout_accounts() {
        let job_id = [5_u8; 32];
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let w2 = Pubkey::new_unique();
        let job = test_job(&[w0, w1, w2], 2, job_id);
        let h1 = [10_u8; 32];
        let h2 = [20_u8; 32];
        let results = vec![
            test_result(job_id, w0, h1),
            test_result(job_id, w1, h1),
            test_result(job_id, w2, h2),
        ];

        let mut payout_candidates = HashSet::new();
        payout_candidates.insert(w0);
        payout_candidates.insert(w1);
        let parsed = finalize_inputs_from_results(&job, &results, &payout_candidates).unwrap();
        assert_eq!(parsed.winning_output_hash, h1);
        assert_eq!(parsed.winner_keys.len(), 2);
        assert!(parsed.winner_keys.contains(&w0));
        assert!(parsed.winner_keys.contains(&w1));
        assert_eq!(parsed.winner_accounts.len(), 2);

        payout_candidates.remove(&w1);
        assert!(finalize_inputs_from_results(&job, &results, &payout_candidates).is_err());
    }

    #[test]
    fn finalize_inputs_rejects_tie_or_no_quorum() {
        let job_id = [6_u8; 32];
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let w2 = Pubkey::new_unique();
        let w3 = Pubkey::new_unique();
        let tie_job = test_job(&[w0, w1, w2, w3], 2, job_id);
        let h1 = [1_u8; 32];
        let h2 = [2_u8; 32];
        let tie_results = vec![
            test_result(job_id, w0, h1),
            test_result(job_id, w1, h1),
            test_result(job_id, w2, h2),
            test_result(job_id, w3, h2),
        ];
        let mut tie_candidates = HashSet::new();
        tie_candidates.insert(w0);
        tie_candidates.insert(w1);
        tie_candidates.insert(w2);
        tie_candidates.insert(w3);
        assert!(finalize_inputs_from_results(&tie_job, &tie_results, &tie_candidates).is_err());

        let strict_job = test_job(&[w0, w1, w2], 3, job_id);
        let no_quorum = vec![test_result(job_id, w0, h1), test_result(job_id, w1, h1)];
        let mut strict_candidates = HashSet::new();
        strict_candidates.insert(w0);
        strict_candidates.insert(w1);
        strict_candidates.insert(w2);
        assert!(finalize_inputs_from_results(&strict_job, &no_quorum, &strict_candidates).is_err());
    }

    #[test]
    fn finalize_inputs_rejects_duplicate_unassigned_or_wrong_job() {
        let job_id = [7_u8; 32];
        let other_job = [8_u8; 32];
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let outsider = Pubkey::new_unique();
        let job = test_job(&[w0, w1], 2, job_id);
        let h = [42_u8; 32];
        let mut candidates = HashSet::new();
        candidates.insert(w0);
        candidates.insert(w1);
        candidates.insert(outsider);

        let duplicate = vec![test_result(job_id, w0, h), test_result(job_id, w0, h)];
        assert!(finalize_inputs_from_results(&job, &duplicate, &candidates).is_err());

        let unassigned = vec![test_result(job_id, outsider, h)];
        assert!(finalize_inputs_from_results(&job, &unassigned, &candidates).is_err());

        let wrong_job = vec![test_result(other_job, w0, h)];
        assert!(finalize_inputs_from_results(&job, &wrong_job, &candidates).is_err());
    }

    #[test]
    fn derive_job_committee_and_quorum_respects_tiering_and_clamps() {
        let mut cfg = test_config();
        cfg.committee_tiering_enabled = false;
        cfg.committee_size = 4;
        cfg.quorum = 5;
        assert_eq!(derive_job_committee_and_quorum(&cfg, 50_000_000), (4, 4));

        cfg.committee_tiering_enabled = true;
        cfg.max_committee_size = 6;
        assert_eq!(derive_job_committee_and_quorum(&cfg, 2_000_000_000), (6, 5));

        cfg.max_committee_size = 4;
        assert_eq!(
            derive_job_committee_and_quorum(&cfg, 20_000_000_000),
            (4, 4)
        );
    }

    #[test]
    fn compute_finalize_payouts_handles_remainder_and_invalid_winner_count() {
        let (protocol_fee, payout_each, remainder) =
            compute_finalize_payouts(1_000_000_001, 100, 3).unwrap();
        assert_eq!(protocol_fee, 10_000_000);
        assert_eq!(payout_each, 330_000_000);
        assert_eq!(remainder, 1);

        assert!(compute_finalize_payouts(100, 100, 0).is_err());
        assert!(compute_finalize_payouts(1_000_000, 10_000, 1).is_ok());
        assert!(compute_finalize_payouts(u64::MAX, 2, 1).is_err());
    }

    #[test]
    fn reset_job_after_no_da_clears_da_fields_and_sets_deadline() {
        let mut job = test_job(&[], 1, [9_u8; 32]);
        job.winning_output_hash = [7_u8; 32];
        job.quorum_reached_slot = 77;
        job.da_deadline_slot = 88;
        job.status = JobStatus::AwaitingDa as u8;

        reset_job_after_no_da(&mut job, 1_000, 42).unwrap();
        assert_eq!(job.winning_output_hash, [0_u8; 32]);
        assert_eq!(job.quorum_reached_slot, 0);
        assert_eq!(job.da_deadline_slot, 0);
        assert_eq!(job.deadline_slot, 1_042);
        assert_eq!(job.status, JobStatus::Assigned as u8);

        assert!(reset_job_after_no_da(&mut job, u64::MAX, 1).is_err());
    }

    #[test]
    fn assign_workers_request_validator_covers_core_rejections() {
        let mut cfg = test_config();
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let w2 = Pubkey::new_unique();
        let mut job = test_job(&[w0, w1, w2], 2, [10_u8; 32]);
        job.status = JobStatus::Posted as u8;
        job.committee_size = 3;

        assert!(validate_assign_workers_request(&job, &cfg, &[w0, w1, w2], 3).is_ok());
        assert!(validate_assign_workers_request(&job, &cfg, &[w0, w1], 2).is_err());
        assert!(validate_assign_workers_request(&job, &cfg, &[w0, w1, w2], 2).is_err());
        assert!(validate_assign_workers_request(&job, &cfg, &[w0, w1, w1], 3).is_err());

        cfg.committee_tiering_enabled = true;
        job.seed_set = false;
        assert!(validate_assign_workers_request(&job, &cfg, &[w0, w1, w2], 3).is_err());
        job.seed_set = true;
        assert!(validate_assign_workers_request(&job, &cfg, &[w0, w1, w2], 3).is_ok());

        job.status = JobStatus::Assigned as u8;
        assert!(validate_assign_workers_request(&job, &cfg, &[w0, w1, w2], 3).is_err());
    }

    #[test]
    fn submit_result_request_validator_covers_state_assignment_and_deadline() {
        let w0 = Pubkey::new_unique();
        let w1 = Pubkey::new_unique();
        let outsider = Pubkey::new_unique();
        let mut job = test_job(&[w0, w1], 2, [11_u8; 32]);
        job.status = JobStatus::Assigned as u8;
        job.deadline_slot = 100;
        assert!(validate_submit_result_request(&job, &w0, 100).is_ok());
        assert!(validate_submit_result_request(&job, &outsider, 10).is_err());
        assert!(validate_submit_result_request(&job, &w0, 101).is_err());
        job.status = JobStatus::Posted as u8;
        assert!(validate_submit_result_request(&job, &w0, 10).is_err());
    }

    #[test]
    fn cancel_expired_request_validator_covers_auth_state_and_expiry() {
        let mut cfg = test_config();
        let caller = Pubkey::new_unique();
        let scheduler = Pubkey::new_unique();
        cfg.scheduler_authority = scheduler;

        let mut job = test_job(&[], 1, [12_u8; 32]);
        job.client = caller;
        job.status = JobStatus::Assigned as u8;
        job.deadline_slot = 100;

        assert!(validate_cancel_expired_request(&job, &cfg, caller, 101).is_ok());
        assert!(validate_cancel_expired_request(&job, &cfg, scheduler, 101).is_ok());
        assert!(validate_cancel_expired_request(&job, &cfg, Pubkey::new_unique(), 101).is_err());
        assert!(validate_cancel_expired_request(&job, &cfg, caller, 100).is_err());
        job.status = JobStatus::Posted as u8;
        assert!(validate_cancel_expired_request(&job, &cfg, caller, 101).is_err());
    }
}
