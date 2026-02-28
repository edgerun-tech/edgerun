# INTENT_UI_GITHUB_PAT_USER_OWNED_V1

## Goal
Make GitHub integration PAT-only and user-owned only:
1. Remove platform connector mode for GitHub.
2. Use Personal Access Token for GitHub auth.
3. Persist GitHub token in encrypted profile secret context.

## Non-Goals
- OAuth/device flow for GitHub.
- Changes to other provider ownership modes.

## Security And Constraints
- GitHub token must not be logged or emitted in event payloads.
- Token persistence should target encrypted profile secrets.
- If profile runtime is not loaded, GitHub connect must fail closed.

## Design
- Update integration catalog for `github`:
  - `authMethod: token`
  - `supportsPlatformConnector: false`
  - user-owned as only mode.
- Remove legacy GitHub `oidc_` token handling.
- In connect path, require loaded profile runtime for GitHub token persistence via `setProfileSecret`.
- Keep integrations event-intent architecture unchanged.

## Acceptance Criteria
- GitHub card/dialog no longer exposes platform ownership mode.
- GitHub setup flow omits the Mode step entirely.
- GitHub connects with PAT only.
- Connected GitHub state derives from profile secret token.
- GitHub filesystem provider reads token from encrypted profile secrets (no localStorage fallback).
- GitHub Values step explicitly states PAT storage in encrypted profile secrets.
- Existing integrations tests updated and pass.

## Rollout
- Frontend-only rollout.

## Rollback
- Revert integration catalog/store changes and related tests.
