# 2026-02-24 Remove Remaining Runtime Mocks V1

## Goal
Remove remaining runtime-visible mock/fake execution behaviors in Cloud OS and frontend control WS paths.

## Non-goals
- Build new backend services for terminal or filter operations.
- Redesign UI architecture.

## Security and Constraint Requirements
- Runtime UI must not fabricate command output, filesystem content, or filtered results.
- Test-only control WS mock hooks must not be available in normal runtime.

## Acceptance Criteria
1. `cloud-os/src/components/Terminal.tsx` no longer returns mock command/file/git/npm/node outputs.
2. `cloud-os/src/components/IntentBar.tsx` filter mode no longer injects mock results.
3. `frontend/lib/scheduler-control-ws.ts` mock resolver only works in Cypress runtime.
4. Generated worker artifact for terminal no longer includes old mock fallback implementation.

## Rollout and Rollback
- Rollout by deploying updated frontend/cloud-os bundles.
- Rollback by reverting this commit set.
