# Device Capability Scheduler Eligibility V1

## Goal
- Extend capability reporting to support scheduler decisions directly.
- Include quantitative resources (totals and allocatable headroom), report freshness metadata, and deterministic job eligibility evaluation.
- Preserve separation: `no_std` core owns schema and evaluation logic; `std` host adapter populates runtime values.

## Non-goals
- No cluster-wide reservation ledger in this phase.
- No external telemetry backend integration in this phase.
- No vendor-specific accelerator metrics beyond count-level availability in this phase.

## Security and constraint requirements
- Probing remains read-only and side-effect free.
- Policy/permission gates must affect effective availability results.
- Missing metrics must be explicit (`Unknown` or `None`), never silently fabricated.
- Eligibility logic must be deterministic and explainable via blocker bitmask.

## Acceptance criteria
- Core report includes domain triplets for `detected`, `available`, `in_use`.
- Core report includes quantitative metrics fields for at least:
  - CPU logical cores (total + allocatable),
  - RAM bytes (total + available),
  - storage bytes (total + available),
  - GPU count (total + available),
  - network/interface and peripheral counts (USB/input/output).
- Core report includes best-effort utilization metrics where supported (CPU load ratio, RAM used bytes, storage used bytes, network links up, GPU busy percent when exposed by driver).
- Core report includes freshness metadata (`collected_unix_s`, `ttl_s`).
- Core report includes diagnostics for consistency:
  - per-domain-field confidence for `detected`/`available`/`in_use`,
  - per-domain-field probe error code (explicit `None` when no error).
- Core exports `JobRequirements` and eligibility evaluator returning:
  - `eligible` boolean,
  - blocker bitmask for precise failure reasons.
- Host Linux adapter populates metrics and metadata with best-effort runtime probing.
- Host Linux adapter sets unsupported or non-portable utilization signals explicitly to `None`/`Unknown` rather than inferring values.
- Host Linux adapter populates diagnostics deterministically from probe outcomes (no opaque failures).
- Unit tests cover evaluator behavior and key parser logic.

## Rollout
1. Add core schema + evaluator.
2. Populate host metrics/metadata on Linux and preserve fallback for non-Linux.
3. Integrate scheduler to consume eligibility result and blocker mask.

## Rollback
- Remove new schema/evaluator fields and revert scheduler integration to previous capability checks.

## Spec alignment notes
- Builds directly on:
  - `2026-02-28-device-capability-probing-core-v1.md`
  - `2026-02-28-device-capability-host-linux-v1.md`
