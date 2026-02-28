# OIDC Scope Canonicals And Intent Gate V1

## Goal
- Define canonical Edgerun OIDC scope strings in one place for runtime/proto consumers.
- Use OIDC scope requirements (not ad-hoc window IDs) for Intent UI capability gating.
- Keep ephemeral mode zero-scope and fail-closed for profile-sensitive capability surfaces.

## Non-goals
- No external OIDC provider integration in this change.
- No token exchange, refresh handling, or backend introspection.
- No full policy engine replacement; only scope checks in Intent UI gate path.

## Security and constraints
- Ephemeral mode grants no OIDC scopes.
- Scope checks are fail-closed: missing required scopes lock capability surfaces.
- Canonical scope names must be deterministic and stable across Rust/frontend.
- Profile mode may grant a baseline local scope set, but admin scopes remain excluded by default.

## Acceptance criteria
- Canonical scope constants exist in `edgerun-runtime-proto` Rust crate.
- Canonical scope constants exist in Intent UI frontend and are used by gate logic.
- Intent UI window gating evaluates scope requirements via `required_all` / `required_any` semantics.
- Existing profile bootstrap Cypress flow remains green.
- Rust + frontend validation commands pass.

## Rollout and rollback
- Rollout: additive constants + gate evaluator replacement.
- Rollback: restore previous hardcoded profile-gated window list and remove scope evaluator wiring.
