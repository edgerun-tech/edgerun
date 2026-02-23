# Anchor Verify Bun Compatibility V1

## Goal
- Keep `program/Anchor.toml` authoritative with `package_manager = "bun"` while allowing `edgerun-cli ci --job verify` to complete on Anchor CLI versions that do not accept `bun` as a package manager variant.

## Non-goals
- No migration to npm, pnpm, or yarn workflows.
- No changes to on-chain program logic, IDL schema, or deployment targets.
- No change to repository-wide package manager preference.

## Security and constraints
- Any compatibility adjustment must be temporary and local to command execution.
- `program/Anchor.toml` must be restored to its original content after verify flow exits.
- The flow must fail fast if it cannot safely read/write/restore the config file.

## Acceptance criteria
- `cargo run -p edgerun-cli -- --root . ci --job verify` succeeds in a Bun-configured workspace.
- `program/Anchor.toml` remains unchanged after the command completes.
- Existing verify stages (`build-sbf`, `idl build`, `anchor test --skip-build`) still execute in sequence.

## Rollout and rollback
- Rollout: add a narrow compatibility guard in `run_program_anchor_verify_sync` that rewrites unsupported `package_manager = "bun"` line only for the duration of anchor CLI subprocesses.
- Rollback: remove the guard once Anchor CLI natively supports `bun` package manager values.
