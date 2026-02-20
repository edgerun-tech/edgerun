# External Security Review Packet

Last updated: 2026-02-20

## Purpose

This document defines the required external security review sign-off package for `crates/edgerun-runtime`.

## Required deliverables

1. Independent review report (scope + methodology + findings).
2. Findings ledger updates in `SECURITY_FINDINGS.json`.
3. Remediation evidence for each closed finding (commit(s), tests, or rationale for accepted risk).
4. Final sign-off metadata (reviewer/org/date) recorded in `SECURITY_FINDINGS.json`.

## Scope checklist

- Bundle decoding and canonicalization
- Wasm validation policy
- Hostcall boundary safety (pointer/length/overflow handling)
- Resource limits (fuel/memory/output)
- Determinism and replay behavior
- Scheduler-assignment policy verification boundary
- Dependency and supply-chain controls

## Sign-off policy

Production sign-off requires all of the following:

- `SECURITY_FINDINGS.json.status == "completed"`
- Sign-off fields populated (`reviewer`, `organization`, `date`)
- No unresolved `high` or `critical` findings

Notes:

- `medium`/`low` findings may remain only with explicit `accepted_risk` status and rationale.
- Any new `high`/`critical` finding re-opens sign-off until resolved.
