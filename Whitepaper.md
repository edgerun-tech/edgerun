DOCUMENT VERSION: edgerun-phase1-spec v1.0
SCOPE: Phase 1 bonded redundant execution (no fraud-proof VM, no rollup).
CHAIN: Solana L1 program for escrow + stake + payout + slashing.
EXECUTION: deterministic WASM runtime + restricted hostcalls.
VERIFICATION: N-of-M redundant results + cryptographic attestations + slashing on contradiction.

DEFINITIONS

Job: A request to execute a WASM module against an input payload under a specific runtime version, producing an output payload.
Bundle: The immutable job payload containing WASM bytecode + input bytes + metadata.
BundleHash: blake3(bundle_bytes) (32 bytes).
OutputHash: blake3(output_bytes) (32 bytes).
ResultDigest: blake3(job_id || bundle_hash || output_hash || runtime_id) (32 bytes).
Attestation: Ed25519 signature by worker over ResultDigest.

Committee: the set of workers assigned to a job for redundant execution.
Quorum: required number of matching attestations on the same OutputHash.

All hashes use BLAKE3-256. All signatures use Ed25519.

RUNTIME (DETERMINISTIC WASM) SPEC

2.1 Determinism constraints (MUST)

WASM target: wasm32

No floating point instructions allowed in modules. Reject at load time if any FP opcode present.

No nondeterministic hostcalls: no time, no rng, no filesystem, no networking, no threads.

Memory is deterministic. Single-threaded.

Maximum linear memory: Configured per job, capped by global max in on-chain config.

Maximum instruction count: Configured per job, capped by global max in on-chain config.

2.2 Allowed imports (ONLY)
The module may import only from module name edgerun:

edgerun.input_len() -> i32

edgerun.read_input(ptr: i32, len: i32) -> i32 (returns bytes read, must equal len)

edgerun.write_output(ptr: i32, len: i32) -> i32 (returns bytes written, must equal len)

No other imports are permitted. Reject bundle if any other import exists.

2.3 Entry point (MUST)
Module MUST export _start() -> (). Runtime calls _start once.

2.4 Input/Output contract (MUST)

Input is provided as raw bytes. Module reads via input_len/read_input.

Output is produced by calling write_output exactly once.

If module calls write_output multiple times or not at all: job fails (JobResult = Failed, no payout, committee replaced; see timeouts).

2.5 Runtime ID (MUST)
Runtime is versioned by runtime_id: [u8;32] which is the BLAKE3 hash of the runtime binary build artifact (or a canonical string + build hash).
Workers MUST refuse jobs with unknown runtime_id.
Scheduler MUST assign only workers advertising support for that runtime_id.

BUNDLE FORMAT (CANONICAL)

Bundle bytes are CBOR (canonical encoding) of:

{
"v": 1,
"runtime_id": bytes32,
"wasm": bytes,
"input": bytes,
"limits": {
"max_memory_bytes": u32,
"max_instructions": u64
},
"meta": {
"content_type": tstr (optional),
"note": tstr (optional)
}
}

Canonicalization: RFC 8949 canonical CBOR (deterministic). BundleHash is BLAKE3(bundle_bytes).

Workers verify:

canonical CBOR decodes

v == 1

runtime_id matches assigned

wasm passes import/FP validation

limits within global caps

bundle_hash equals on-chain bundle_hash

Storage layer must serve the exact bundle_bytes that hash to bundle_hash.

NETWORK / OFF-CHAIN COMPONENTS

4.1 Components
A) Scheduler API (centralized MVP)
B) Worker daemon (installed by providers)
C) Storage service (your own) for bundles

Solana program is chain-side settlement only.

4.2 Scheduler responsibilities (MUST)

Maintain worker registry + heartbeats + capability set (runtime_ids supported)

Assign committees

Track job timeouts and reassignments

Provide bundle download pointers

Observe chain events (job posted, results submitted, finalized)

Trigger finalization transaction when quorum achieved or job expired

4.3 Worker daemon responsibilities (MUST)

Maintain a Solana keypair for staking + signing attestations

Heartbeat to scheduler

Fetch assigned jobs, download bundle, verify bundle_hash, execute, compute output_hash

Produce attestation signature over ResultDigest

Submit on-chain result (transaction)

Serve as evidence provider if asked (bundle / logs), but MVP does not require proofs beyond signatures

SOLANA PROGRAM SPEC (ANCHOR-FRIENDLY)

5.1 PDAs (Program Derived Addresses)

GlobalConfig PDA: seeds ["config"]

Treasury PDA (optional): seeds ["treasury"]

WorkerStake PDA: seeds ["worker_stake", worker_pubkey]

Job PDA: seeds ["job", job_id_bytes] where job_id_bytes is 32 bytes.

JobResult PDA (optional; recommended for account sizing): seeds ["job_result", job_id, worker_pubkey]

To minimize account bloat, prefer JobResult PDAs so the Job account stays fixed-size.

5.2 Token model
MVP uses native SOL for escrow and stake (simpler). You can switch to SPL EDGE token later; spec allows either with a config flag. Start with SOL.

5.3 Accounts

GlobalConfig (fixed size)

admin: Pubkey

scheduler_authority: Pubkey (the off-chain scheduler signer for assign/finalize ops, MVP)

min_worker_stake_lamports: u64

min_challenger_stake_lamports: u64 (MVP can set 0 if no public challenges)

protocol_fee_bps: u16 (e.g., 300 = 3.00%)

committee_size: u8 (default 3)

quorum: u8 (default 2)

challenge_window_slots: u64 (default e.g., 900 slots ~ 6–8 min; tune)

max_memory_bytes: u32 (global cap)

max_instructions: u64 (global cap)

allowed_runtime_root: [u8;32] (Merkle root of allowed runtime_ids; MVP can allow any and set to 0)

paused: bool

WorkerStake (per worker)

worker: Pubkey

total_stake_lamports: u64

locked_stake_lamports: u64

reputation: i32 (optional, off-chain primary; on-chain optional)

status: u8 (0 active, 1 jailed)

Job (per job)

job_id: [u8;32]

client: Pubkey

escrow_lamports: u64

bundle_hash: [u8;32]

runtime_id: [u8;32]

max_memory_bytes: u32

max_instructions: u64

committee_size: u8

quorum: u8

created_slot: u64

deadline_slot: u64 (created_slot + challenge_window_slots)

assigned_workers: [Pubkey; 3] (committee_size fixed to 3 for MVP; if you want variable, use Vec and dynamic sizing)

status: u8

0 Posted

1 Assigned

2 Finalized

3 Cancelled

4 Slashed

JobResult (per worker per job) (recommended)

job_id: [u8;32]

worker: Pubkey

output_hash: [u8;32]

attestation_sig: [u8;64] (Ed25519 signature)

submitted_slot: u64

Rationale: Job stores only assignment + parameters. Results are in separate small accounts.

5.4 Instruction set (complete)

(1) initialize_config(admin, scheduler_authority, params...)

Creates GlobalConfig PDA

Sets fields

Only callable once

(2) update_config(params...)

admin-only

(3) register_worker_stake()

Creates WorkerStake PDA for signer

Initializes status active

(4) deposit_stake(amount_lamports)

Transfers SOL from worker to WorkerStake PDA (or escrow vault PDA) and increments total_stake

Must keep rent-exemption rules satisfied

(5) withdraw_stake(amount_lamports)

Only allowed if (total_stake - amount) >= locked_stake and status active

Transfers SOL back to worker

(6) post_job(job_id, bundle_hash, runtime_id, limits, committee_size=3, quorum=2)

Validates limits <= global caps

Requires escrow payment transfer from client to Job PDA (escrow_lamports)

Creates Job PDA with status Posted

deadline_slot = current_slot + challenge_window_slots

committee_size/quorum fixed to config defaults unless you allow override; MVP uses config defaults

Emits event JobPosted(job_id,...)

(7) assign_workers(job_id, workers[3])

Only scheduler_authority can call

Job must be Posted

For each worker:

WorkerStake must exist, active

total_stake >= min_worker_stake

lock stake: locked_stake += required_lock (defined below)

Writes assigned_workers

Sets status Assigned

Emits event JobAssigned(job_id, workers)

Required lock per worker:

required_lock = max(min_worker_stake, escrow_lamports * STAKE_MULTIPLIER_NUM / STAKE_MULTIPLIER_DEN / committee_size)
MVP constants in config:

stake_multiplier_bps (e.g., 30000 = 3x). For simplicity: required_lock = max(min_worker_stake, escrow/ quorum) * 2, but pick one and freeze it.
Freeze this now:

required_lock = max(min_worker_stake, escrow_lamports * 3 / 2 / committee_size)
Explanation: locks 1.5x escrow spread across committee; adjust later. The exact formula matters less than being deterministic. This is deterministic.

(8) submit_result(job_id, output_hash, attestation_sig)

Callable only by assigned worker signer

Job must be Assigned and not past deadline_slot (or allow late submissions but no guarantee)

Verifies signature off-chain? On-chain Ed25519 verification is possible via Solana ed25519 program instruction. MVP SHOULD REQUIRE ed25519 verification.

ResultDigest = blake3(job_id || bundle_hash || output_hash || runtime_id)

Verify attestation_sig is valid Ed25519 signature by worker over ResultDigest.
Implementation detail:

Include an Ed25519Program instruction in the same transaction and verify via CPI/sysvar instructions. Anchor pattern: read instructions sysvar.

Creates JobResult PDA for (job_id, worker) storing output_hash + signature + submitted_slot

Emits event JobResultSubmitted(job_id, worker, output_hash)

(9) finalize_job(job_id)

Callable by scheduler_authority (MVP) OR permissionless (recommended once stable)

Job must be Assigned

Reads JobResult PDAs for assigned workers (some may be missing)

Computes if any output_hash has >= quorum matching results

If yes:

Determine winning_output_hash

Mark losers: any submitted result != winning_output_hash is slashable; any missing result is timeout-penalizable (see penalties)

Pay out:

protocol_fee = escrow * protocol_fee_bps / 10000

worker_pool = escrow - protocol_fee

each winner gets worker_pool / quorum_winners_count (or divide by committee_size to keep predictable; freeze policy now)
Freeze payout policy now:

Pay equally to winners only (those who submitted winning hash).

If quorum=2 and 2 winners: each gets worker_pool/2.

If 3 winners: each gets worker_pool/3.

If only 2 submitted and match: still quorum and winners=2.

If only 2 submitted and mismatch: no finalize.

Unlock stake for winners immediately: locked_stake -= required_lock.

For losers:

Slash rule below (MVP can just “mark slashable” and scheduler calls slash instruction)

Transfer protocol_fee to treasury (admin-controlled account) or config.treasury

Set job status Finalized

Emits JobFinalized(job_id, winning_output_hash)

(10) slash_worker(job_id, worker)

Callable by scheduler_authority (MVP) OR permissionless if evidence exists

Evidence: worker submitted a JobResult with output_hash != winning_output_hash recorded in JobFinalized event/state.

Slashing amount:
Freeze slashing now:

slash_amount = required_lock (the locked amount for that job)

distribute: 50% to honest winners (pro-rata), 25% to treasury, 25% burned (sent to incinerator address or kept in treasury; SOL burn is tricky; prefer treasury)
MVP distribution (simple):

75% treasury, 25% to winners equally (or all treasury).
Pick one and freeze:

100% treasury for MVP (simplest). Winners already got paid.

Decrement worker total_stake by slash_amount and locked_stake by required_lock.

Optionally set worker status jailed if total_stake < min or too many slashes (keep off-chain for MVP)

(11) cancel_expired_job(job_id)

Callable by scheduler_authority or client after deadline_slot if no quorum

If no quorum reached by deadline:

refund escrow to client minus protocol fee? Freeze policy now.
Freeze cancellation now:

Full refund to client (no protocol fee) if job not finalized.

Unlock any locked stakes for workers who did not submit contradictory results (if they submitted something but no quorum, no slashing in Phase 1)

Set status Cancelled

That’s the complete on-chain API for Phase 1.

OFF-CHAIN API SPEC (SCHEDULER)

Transport: HTTPS JSON for MVP. (You can later move to gRPC.)

6.1 Worker registration / heartbeat

POST /v1/worker/heartbeat
Request:
{
"worker_pubkey": "base58",
"runtime_ids": ["hex32", ...],
"version": "worker-daemon semver",
"capacity": {
"max_concurrent": 2,
"mem_bytes": 268435456
},
"signature": "base64 ed25519 over blake3(body_without_signature)"
}

Response:
{ "ok": true, "server_time": "...", "next_poll_ms": 2000 }

6.2 Poll for assignments

GET /v1/worker/assignments?worker_pubkey=...
Response:
{
"jobs": [
{
"job_id": "hex32",
"bundle_hash": "hex32",
"bundle_url": "https://storage/.../bundle.cbor",
"runtime_id": "hex32",
"limits": { "max_memory_bytes": 268435456, "max_instructions": 500000000 },
"submit_deadline_slot": 123456789,
"chain": { "cluster": "mainnet-beta", "program_id": "...", "job_pda": "..."}
}
]
}

6.3 Submit execution result to scheduler (optional, for monitoring)
Worker still submits to chain directly; scheduler endpoint is for observability.

POST /v1/worker/result
{
"worker_pubkey": "base58",
"job_id": "hex32",
"output_hash": "hex32",
"attestation_sig": "base64",
"logs": "optional short string",
"signature": "base64 signer over blake3(body_without_signature)"
}

6.4 Client job creation

POST /v1/job/create
Request:
{
"runtime_id": "hex32",
"wasm_base64": "...",
"input_base64": "...",
"limits": { "max_memory_bytes": 268435456, "max_instructions": 300000000 },
"escrow_lamports": 10000000
}

Response:
{
"job_id": "hex32",
"bundle_hash": "hex32",
"bundle_url": "...",
"post_job_tx": "base64 serialized solana tx (partially signed by server if needed or unsigned)",
"instructions": ["client signs and sends post_job_tx"]
}

MVP simplification: scheduler generates job_id, builds bundle, stores it, returns bundle_hash and the client posts job on-chain.

WORKER DAEMON EXECUTION PIPELINE (DETERMINISTIC)

Algorithm:

poll assignments

download bundle_bytes

compute bundle_hash = blake3(bundle_bytes), must match assignment and on-chain job.bundle_hash

decode canonical CBOR, validate imports and no FP

execute with instruction limit and memory cap

capture output_bytes from single write_output call

compute output_hash

compute result_digest = blake3(job_id || bundle_hash || output_hash || runtime_id)

signature = ed25519_sign(worker_key, result_digest)

submit_result(job_id, output_hash, signature) to chain in a tx that also includes ed25519 verify instruction (standard Solana pattern)

Timeout policy:

If execution exceeds max wall time (local safety, e.g., 60s), abort and do not submit. Scheduler may reassign. (No slashing for “no-submit” in MVP; later you can add penalties.)

SECURITY MODEL (PHASE 1)

8.1 What is cryptographically enforced

Workers cannot deny what they signed.

Contradictory results are objective (different output_hash values) and tied to a single job/bundle/runtime_id.

Quorum rule is deterministic.

8.2 What is not yet trustless

If committee colludes and returns the same wrong hash, Phase 1 cannot detect it. Mitigation: committee selection + stake requirements + optional “verifier committee” in Phase 2.

8.3 Anti-griefing

Only assigned workers can submit results.

finalize_job can be called by scheduler authority at first to reduce spam.

Later make finalize permissionless after hardening.

ECONOMICS (FROZEN FOR MVP)

Payment currency: SOL

protocol_fee_bps: 300 (3%)

committee_size: 3

quorum: 2

stake lock multiplier: embedded in required_lock formula above

slashing: only for submitting a non-winning output_hash once quorum finalizes

missing submission: no slashing in MVP (handled by scheduler replacement + off-chain reputation)

This prints money because: protocol fee accrues on each finalized job.

TEST PLAN (MUST PASS BEFORE MAINNET)

10.1 Determinism tests

Run the same bundle on 10 machines and confirm identical output_hash.

Randomized fuzz inputs for 1,000 runs.

10.2 On-chain tests (Anchor)

post_job escrow transfer correctness

assign_workers locks stake

submit_result rejects bad signature

submit_result rejects non-assigned worker

finalize_job:

quorum 2-of-3 success

only 2 submissions matching success

mismatch no finalization

cancel_expired_job refunds escrow and unlocks stake

slash_worker reduces stake

10.3 Integration tests

End-to-end: client → scheduler → workers → chain → finalize.

Fault injection: one worker returns wrong output. Confirm slash path.

PRODUCTION DEPLOYMENT CHECKLIST

Solana program deployed (devnet -> mainnet)

Scheduler runs with:

chain watcher (websocket)

job state database

worker heartbeat store

Storage service:

content-addressable by bundle_hash

immutable objects

Worker daemon packaging:

single static binary

auto-update channel pinned by hash (optional)

Monitoring:

job latency

worker failure rate

mismatch rate

treasury fee inflow

WHAT YOUR AI AGENT SHOULD IMPLEMENT (ORDERED TODO LIST)

This is the single linear list from 0 → production.

A) Solana program (Anchor)

Create Anchor workspace edgerun_program

Implement GlobalConfig account + initialize/update

Implement WorkerStake + register/deposit/withdraw

Implement Job account + post_job

Implement assign_workers with stake locking

Implement JobResult PDA + submit_result

Implement ed25519 signature verification via instructions sysvar

Implement finalize_job quorum logic + payouts + stake unlock

Implement cancel_expired_job

Implement slash_worker

Write full Anchor tests for all paths

B) Runtime
12. Create edgerun-runtime crate/binary
13. Implement bundle CBOR canonical parsing and hashing
14. WASM validation: reject FP opcodes, reject non-edgerun imports
15. Implement hostcalls: input_len/read_input/write_output
16. Implement instruction limiting + memory limiting
17. Produce output_bytes and output_hash
18. Expose CLI: edgerun-runtime run --bundle bundle.cbor --output out.bin
19. Determinism test harness across multiple runs

C) Worker daemon
20. Create edgerun-worker binary
21. Implement Solana key management + stake management commands
22. Implement scheduler heartbeat + assignment polling
23. Implement bundle fetch + verification + runtime invocation
24. Implement chain submit_result tx builder with ed25519 verify instruction
25. Implement local queue for concurrent jobs + max_concurrent cap
26. Add structured logs + crash-safe restart

D) Scheduler
27. Create edgerun-scheduler service
28. Implement worker registry + heartbeat expiry
29. Implement job create endpoint (bundle build + store + return job_id/bundle_hash)
30. Implement chain watcher: detect post_job, move to assignment
31. Implement committee selection (random among eligible staked workers supporting runtime_id)
32. Call assign_workers tx with scheduler_authority
33. Track results (watch JobResult submissions)
34. Call finalize_job tx when quorum met
35. Call cancel_expired_job after deadline if no quorum
36. Optionally call slash_worker after finalization for losers

E) Storage
37. Implement content-addressed blob store keyed by bundle_hash
38. Provide GET /bundle/{bundle_hash} returning exact bundle_bytes
39. Ensure immutability + retention at least for challenge_window duration

F) Launch
40. Deploy devnet end-to-end
41. Run load test: 10k jobs small bundles
42. Deploy mainnet program + scheduler + storage
43. Recruit initial workers, set min stake, and start charging clients

FROZEN CHOICES (NO ROOM FOR ERROR)

Hash: BLAKE3-256 everywhere

Signature: Ed25519

Execution: wasm32 deterministic, no FP, only edgerun hostcalls

Committee: 3

Quorum: 2

Currency: SOL (MVP)

Results: output_hash only; no raw output on-chain

Evidence: signatures + on-chain stored output_hash values

Scheduler authority: centralized for assignment/finalize in MVP