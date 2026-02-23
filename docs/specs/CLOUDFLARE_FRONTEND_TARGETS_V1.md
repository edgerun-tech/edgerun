# Cloudflare Frontend Target Mapping Spec (V1)

## Status
- Proposed and implemented in this change set.

## Goal
- Ensure the two frontend projects deploy to explicit, distinct Cloudflare Workers targets with zero ambiguity.

## Non-goals
- Changing application code behavior for either frontend.
- Introducing Cloudflare Pages migration.
- Altering DNS ownership or account-level Cloudflare settings.

## Security and Constraint Requirements
- Deployment commands must pin an explicit wrangler config path.
- Each frontend target must have a unique worker name.
- Validation must fail if worker names or asset directories collide.
- Repository-default JS workflow remains bun/bunx-based.

## Acceptance Criteria
1. Two explicit wrangler configs exist and are mapped to the two frontends.
2. Deploy commands in scripts/package scripts use `--config <path>`.
3. A repo script verifies non-overlap between frontend targets.
4. Documentation captures the target mapping and verification command.

## Rollout
1. Add cloud-os wrangler config.
2. Pin frontend/cloud-os deploy commands to explicit config files.
3. Add target verification script and wire it into drift/workflow checks.
4. Update compact execution guidelines with verification command.

## Rollback
- Revert explicit config pinning and verification script.
- Restore previous deploy behavior.
