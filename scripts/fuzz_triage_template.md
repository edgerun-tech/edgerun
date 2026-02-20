# Fuzz Crash Triage Template

Date:
Owner:
Target:
Artifact path:
Commit:

## Reproduction

1. Reproduce command:
2. Reproduced locally: yes/no
3. Deterministic reproduction input saved: yes/no

## Impact

1. Surface: decode / validation / hostcall / execution
2. Failure class: panic / OOB / DOS / invariant break / other
3. Security severity: low / medium / high / critical
4. Exploitability notes:

## Fix

1. Root cause:
2. Code change:
3. Added regression test(s):
4. Added corpus seed:

## Verification

1. `cargo test -p edgerun-runtime`:
2. `cargo check --manifest-path crates/edgerun-runtime/fuzz/Cargo.toml`:
3. Target rerun without crash:

## Closure

1. PR/commit:
2. Reviewer:
3. Follow-up actions:
