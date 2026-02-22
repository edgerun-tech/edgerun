# Market Placement Deliverables

## Objective
Establish a production-safe early-stage pricing model for Edgerun based on deterministic execution, not infrastructure analogies.

## Product Positioning Deliverable
### Deliverable
- Publish canonical market positioning language across docs/site/decks.

### Required output
- Replace cloud-cost anchor messaging with deterministic-settlement messaging:
  - "Pay only for deterministic work executed."
  - "Cloud bills machines; Edgerun bills verified execution."

### Acceptance criteria
- No references to "X% cheaper than cloud" in core positioning docs.
- Positioning clearly states deterministic work-unit billing.

## Pricing Primitive Deliverable
### Deliverable
- Define and lock the pricing primitive as Deterministic Work Units (DWU).

### Required output
- DWU definition: instruction-count based unit derived from `max_instructions`.
- Billing input fields locked to:
  - `max_instructions`
  - memory cap (for enforcement, optional multiplier later)
  - committee size / redundancy multiplier

### Acceptance criteria
- Pricing spec explicitly excludes CPU time as a billing primitive.
- Pricing spec is deterministic and hardware-agnostic.

## Early-Stage Billing Formula Deliverable
### Deliverable
- Implement fixed instruction-based pricing for Phase 1.

### Required output
- Formula:
  - `total_cost = (max_instructions / 1e9) * P * C`
  - where `P = base SOL price per 1e9 instructions`, `C = committee size`
- Optional anti-spam flat fee documented.
- Initial constant published (example: `P = 0.01 SOL`).

### Acceptance criteria
- Given identical job params, all nodes compute identical quoted price.
- Formula includes explicit redundancy multiplier via committee size.

## Abuse Prevention Deliverable
### Deliverable
- Enforce economic safety controls before execution.

### Required output
- Mandatory pre-execution escrow.
- Hard instruction limit enforcement.
- Hard memory cap enforcement.
- Fail-on-limit breach semantics (no free overrun).
- No job admission without escrow.

### Acceptance criteria
- Job without escrow is rejected.
- Execution exceeding limits is terminated and reported as failed.

## Rollout Plan Deliverable
### Deliverable
- Publish staged market evolution plan with strict gating.

### Phase 1 (now)
- Fixed instruction pricing only.
- No dynamic bidding.
- No benchmark-based pricing.

### Phase 2 (later)
- Worker-declared performance/reliability tiers for eligibility/scheduling.
- Benchmarking used for reputation and scheduling only.

### Phase 3 (scale)
- Optional worker-declared `price_per_instruction` and scheduler market selection for eligible quorum.

### Acceptance criteria
- Current implementation remains Phase 1 only.
- Future phases documented but not partially enabled.

## Benchmarking Policy Deliverable
### Deliverable
- Define permitted use of benchmarking.

### Required output
- Benchmarking allowed for:
  - reputation scoring
  - eligibility tiers
  - scheduling optimization
- Benchmarking explicitly not used for Phase 1 billing.

### Acceptance criteria
- Pricing path has no runtime dependency on benchmark score.

## Go-To-Market Copy Deliverable
### Deliverable
- Provide approved messaging snippets for external use.

### Required output
- One-liner: "Edgerun is a deterministic compute settlement network."
- Pricing line: "You pay for deterministic work units, multiplied by required redundancy."
- Contrast line: "Cloud pricing sells machine time; Edgerun prices verified execution."

### Acceptance criteria
- Website/docs/investor deck all use consistent copy.

## Decision Log Deliverable
### Deliverable
- Record rationale for choosing deterministic fixed pricing first.

### Required output
- Explicitly list avoided early-stage risks:
  - synthetic market complexity
  - benchmark gaming pressure
  - unstable cloud-comparison narratives

### Acceptance criteria
- Rationale is documented in architecture/economics decision records.

## Definition of Done
- Deliverables above are implemented and published.
- Formula is deterministic and verifiable.
- Escrow + limits prevent compute abuse.
- Positioning reflects settlement-layer economics, not cloud discount framing.

## Execution Status (2026-02-22)
Owner: Codex

| Deliverable | Status | Evidence |
| --- | --- | --- |
| Product positioning language defined | Complete | `frontend/components/landing/hero-section.tsx`, `frontend/components/landing/features-section.tsx` |
| DWU pricing primitive defined | Complete | `DWU` and deterministic billing inputs specified |
| Phase 1 formula defined | Complete | `crates/edgerun-scheduler/src/main.rs` (`required_instruction_escrow_lamports`) |
| Abuse prevention controls defined | Complete | Scheduler rejects underfunded jobs before assignment (`job_create`) |
| Rollout phases defined | Complete | Phase 1/2/3 gating captured |
| Benchmarking policy defined | Complete | Allowed/disallowed uses documented |
| Decision rationale captured | Complete | Risk list and rationale documented |
| Cross-doc replacement of cloud-discount claims | Complete (docs set check) | Search across `docs/` found no conflicting cloud-discount anchor claims |

### Implemented controls and knobs
- Scheduler deterministic pricing env knobs:
  - `EDGERUN_SCHEDULER_LAMPORTS_PER_BILLION_INSTRUCTIONS`
  - `EDGERUN_SCHEDULER_FLAT_FEE_LAMPORTS`
- Admission guard:
  - `job_create` rejects if `escrow_lamports < required_instruction_escrow_lamports(...)`.
- Formula implementation:
  - `required_instruction_escrow_lamports(max_instructions, P, C, flat_fee)` where `P` is lamports per 1e9 instructions and `C` is redundancy multiplier.
- Proof tests:
  - `instruction_escrow_formula_uses_redundancy_and_ceiling`
  - `job_create_rejects_escrow_below_instruction_minimum`
