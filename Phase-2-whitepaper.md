DOCUMENT VERSION: edgerun-phase2-practical-spec v1.0
SCOPE: Phase 2 practical hardening (no fraud-proof VM).
GOAL: Remove scheduler authority dependency for settlement, improve reliability, enforce output availability, and reduce collusion risk via randomness + adaptive committees.
CHAIN: Solana L1 program for escrow + stake + payout + slashing.
OFF-CHAIN: Scheduler remains for UX/throughput but is not trusted for finalization.

FROZEN CHOICES (PHASE 2 PRACTICAL)
Hash: BLAKE3-256 everywhere.
Signature: Ed25519.
Fast path verification: N-of-M redundant results (same as Phase 1).
Finalization: permissionless.
Data availability (DA): winners must publish output via on-chain OutputAvailability commitment before payout.
Liveness: missing submissions penalized (small fixed slash) + reassignment allowed.
Collusion mitigation: randomness-seeded committee selection + escrow-based committee tiering.

1. DEFINITIONS (ADDITIONS)
   JobSeed: bytes32 used to derive committee selection deterministically.
   WinningOutputHash: output_hash that meets quorum.
   OutputAvailability: on-chain commitment that binds job_id -> output_hash + pointer to retrievable output bytes.
   DA Window: slots between quorum reached and da_deadline_slot.
   NonResponsePenalty: fixed lamports slashed for assigned worker who fails to submit by deadline.

All hashes use BLAKE3-256. All signatures use Ed25519.

2. RUNTIME / EXECUTION (UNCHANGED FROM PHASE 1)
   Deterministic wasm32, no FP, only edgerun imports, instruction and memory caps enforced off-chain.
   No new VM proof requirements.

3. STORAGE (PHASE 2 DA REQUIREMENT)
   Storage MUST serve:

* bundle_bytes by bundle_hash (unchanged)
* output_bytes by output_hash (new)

Outputs are content-addressed:
output_url = [https://storage/.../output/{output_hash}](https://storage/.../output/{output_hash})
Pointer stored on-chain is advisory; output_hash is authoritative.

4. SOLANA PROGRAM SPEC (PHASE 2 PRACTICAL)

4.1 GlobalConfig (additions)
Add fields to GlobalConfig:

* randomness_authority: Pubkey
  A signer allowed to set Job.seed. Initially your scheduler key. Later can be VRF/oracle.
* da_window_slots: u64
  Default e.g. 900 slots.
* non_response_slash_lamports: u64
  Default e.g. 50_000 (tune later).
* committee_tiering_enabled: bool
* committee_tiers: fixed array (Anchor-friendly)
  Freeze exact tiers now (see 4.4).
* max_committee_size: u8 (default 9)

Keep existing:
committee_size, quorum as defaults for smallest tier.

4.2 Job account (additions/changes)
Add fields:

* seed: [u8;32] (default all zeros until set)
* seed_set: bool
* winning_output_hash: [u8;32] (zero until quorum)
* quorum_reached_slot: u64 (0 until set)
* da_deadline_slot: u64 (0 until set)
* committee_size: u8 (stored per job, derived from tier)
* quorum: u8 (stored per job, derived from tier)
* status additions:
  0 Posted
  1 Seeded
  2 Assigned
  3 QuorumReached
  4 AwaitingDA
  5 Finalized
  6 Cancelled
  7 Slashed (optional)

Assigned workers storage:

* MVP keep fixed max size 9:
  assigned_workers: [Pubkey; 9]
  assigned_count: u8
  This avoids Vec sizing.

4.3 JobResult (additions)
Add fields:

* bytecode_hash: [u8;32] (keep from Phase 1 digest update if you already added it; otherwise set to 0 and do not include in digest. Freeze one choice below.)
* submitted: bool (or infer by account existence)
  No other required changes.

Digest choice (freeze):
Keep Phase 1 digest:
ResultDigest = blake3(job_id || bundle_hash || output_hash || runtime_id)
Do NOT add bytecode_hash in this practical Phase 2. (Less moving parts, no client-visible gain.)

4.4 Committee tiering (FROZEN)
Committee is derived from escrow_lamports at post_job time:

Tier table (fixed):

* Tier0: escrow < 0.1 SOL (100_000_000 lamports): committee_size=3, quorum=2
* Tier1: 0.1–1 SOL: committee_size=5, quorum=3
* Tier2: 1–10 SOL: committee_size=7, quorum=5
* Tier3: >=10 SOL: committee_size=9, quorum=6

Job stores committee_size and quorum so later logic is deterministic.

4.5 New PDA: OutputAvailability
OutputAvailability PDA: seeds ["output", job_id]
Fields:

* job_id: [u8;32]
* output_hash: [u8;32]
* publisher: Pubkey
* pointer: [u8;128] (fixed bytes; store UTF-8 string truncated/padded with 0)
* published_slot: u64

4.6 Worker set snapshot for deterministic selection (practical approach)
Fully deterministic on-chain sampling from “all active workers” is expensive without an indexed set.

Practical frozen rule for v1.0:

* Committee selection remains off-chain by scheduler for now.
* But finalization is permissionless and DA + penalties are enforced on-chain.
* Scheduler authority is removed from finalize, but still used for assign_workers until you later build a WorkerSet index.

This gives you the big wins (permissionless finalize, DA gating, liveness penalties) without building an on-chain registry data structure.

4.7 Instruction set (Phase 2 practical complete)

(1) set_job_seed(job_id, seed)

* Callable only by config.randomness_authority
* Job must be Posted and seed_set=false
* Sets seed and seed_set=true
* Sets status Seeded

(2) post_job(...) (modified)

* Same as Phase 1, but sets committee_size/quorum based on escrow tier at creation.
* status Posted

(3) assign_workers(job_id, workers[assigned_count])

* Callable by scheduler_authority (same as Phase 1) for v1.0 practical.
* Requires job.seed_set=true (prevents “scheduler picks committee after seeing results” games).
* Locks stake required_lock for each assigned worker.
* Stores assigned_workers + assigned_count.
* status Assigned.

(4) submit_result(job_id, output_hash, attestation_sig)

* Same as Phase 1.
* Only assigned worker may call.
* Ed25519 verify over Phase 1 ResultDigest.
* Creates JobResult PDA.

(5) reach_quorum(job_id)
Permissionless.

* Job must be Assigned and before deadline_slot.
* Reads JobResult PDAs for assigned workers.
* If any output_hash has >= job.quorum matching:

  * sets winning_output_hash
  * quorum_reached_slot = current_slot
  * da_deadline_slot = current_slot + config.da_window_slots
  * status AwaitingDA
  * emits JobQuorumReached(job_id, winning_output_hash)

(6) declare_output(job_id, output_hash, pointer_bytes<=128)
Callable only by a “winner” (a worker whose JobResult.output_hash == job.winning_output_hash).

* Job must be AwaitingDA
* output_hash must equal job.winning_output_hash
* Creates OutputAvailability PDA if absent.
* Stores pointer (padded to 128).
* published_slot = current_slot

(7) finalize_job(job_id)
Permissionless.

* Job must be AwaitingDA
* Requires current_slot <= da_deadline_slot OR allow finalize anytime after declare_output; freeze:

  * Allow finalize anytime after declare_output, but if da_deadline_slot passes without OutputAvailability, job can be re-assigned (see next).
* Requires OutputAvailability exists and matches winning_output_hash.
* Pays out:
  protocol_fee = escrow * fee_bps / 10000
  worker_pool = escrow - protocol_fee
  winners = subset of assigned workers with JobResult.output_hash == winning_output_hash
  payout equally to winners only (same as Phase 1)
* Stake unlock for winners.
* Slashing:

  * wrong submitters (submitted != winning) are slashable: slash_amount = required_lock, send 100% to treasury (same as Phase 1)
  * missing submitters: apply NonResponsePenalty (see 8)
* status Finalized
* emits JobFinalized(job_id, winning_output_hash)

(8) penalize_non_submitters(job_id)
Permissionless.

* Callable when Job is Finalized OR when deadline_slot passed.
* For each assigned worker with no JobResult PDA:

  * slash min(non_response_slash_lamports, available_stake_not_locked) from WorkerStake.total_stake
  * transfer to treasury
    This is intentionally small; do not brick workers for transient failures.

(9) reassign_missing(job_id, missing_worker, replacement_worker)
Callable by scheduler_authority for v1.0 (or permissionless later).

* Job must be Assigned and current_slot > deadline_slot
* missing_worker must be in assigned list and has no JobResult
* replacement_worker must be active + staked + supports runtime_id (checked off-chain for v1.0)
* Unlock missing_worker required_lock (so funds aren’t stuck forever), then apply NonResponsePenalty (or keep penalty in (8))
* Lock stake for replacement_worker, replace in assigned_workers
* status stays Assigned, extend deadline_slot by a fixed extension (freeze: +challenge_window_slots/2)

(10) cancel_no_da(job_id)
Permissionless.

* Job must be AwaitingDA and current_slot > da_deadline_slot and OutputAvailability missing
  Freeze policy:
* Revert to Assigned and allow scheduler to reassign whole committee (preferred), OR cancel/refund.
  Pick and freeze now:
  Revert to Assigned (keeps job alive).
  Implementation:

  * clear winning_output_hash
  * status Assigned
  * extend deadline_slot by challenge_window_slots
  * optionally apply NonResponsePenalty to winners that failed to declare output (small)

That is Phase 2 practical.

5. OFF-CHAIN SCHEDULER (PHASE 2 CHANGES)
   Must do:

* Set seed on-chain immediately after post_job (or include as part of create flow).
* Assign committee only after seed_set=true.
* When quorum reached, ensure a winner publishes output_bytes to storage and calls declare_output.
* Watch da_deadline_slot; if DA missing, call cancel_no_da and reassign.
* Track non-response and reassign_missing.

6. MONEY PRINTING NEXT (SELLING + GTM) — WHAT TO DO RIGHT AFTER PHASE 2
   Do not build more protocol. Sell.

Target buyers (highest probability)

* Web scraping / automation shops: “deterministic WASM jobs at scale, pay-per-job”
* AI data labeling / preprocessing: “run deterministic transforms privately”
* Crypto teams needing off-chain compute with on-chain settlement (simple, not rollup)

Offer (simple SKU)

* API: submit job (wasm+input) -> get output_hash + output_url
* SLA tiers:

  * Bronze: committee 3/2
  * Silver: 5/3
  * Gold: 7/5 or 9/6
    Price:
* protocol_fee_bps + optional per-job minimum fee
* add “priority fee” for low latency (scheduler-level)

Immediate sales assets to produce

1. Landing page: 1 sentence + 3 bullets + “Run a job” demo.
2. SDK: minimal (TS + curl examples).
3. 2 demo workloads:

   * JSON normalization / redaction (privacy angle)
   * Deterministic compression / hashing pipeline (easy to verify)
4. A “trust page” explaining determinism + committee + slashing + DA.

If you want, paste your current repo structure (or just tell me languages used for scheduler/worker/runtime), and I will output the exact Phase 2 implementation task list as PR-sized chunks (each chunk merges cleanly and moves you toward “sellable MVP”).
