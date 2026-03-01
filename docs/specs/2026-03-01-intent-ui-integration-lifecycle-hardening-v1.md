# Intent UI Integration Lifecycle Hardening v1

## Goal
Make integration setup deterministic and event-native so UI state always reflects real lifecycle transitions.

## Non-goals
- Rewriting all integration providers.
- Migrating integration worker build/copy pipeline.
- Introducing backend persistence changes.

## Security and Constraint Requirements
- No token value may be emitted in event payloads.
- Integration status must be derived from event-driven reducer updates.
- Keep compatibility with existing `intent.ui.integration.*` topics.
- Keep JS runtime on Bun toolchain and existing frontend build path.

## Design
- Add explicit lifecycle state per integration in the store:
  - `idle`, `verifying`, `verified`, `linking`, `connected`, `disconnected`, `error`.
- Add lifecycle event topic for consumer panels:
  - `ui.integration.lifecycle.changed`.
- Normalize verification event handling:
  - reducer accepts both intent and event topics for verify success/failure.
- Set lifecycle in reducer actions (`checkAll`, `connect`, `disconnect`) so state transitions are visible without polling.
- Gate link action in integrations dialog on lifecycle `verified` (or already connected), not only ad-hoc local state.

## Acceptance Criteria
1. Running verification updates lifecycle to `verifying`, then `verified`/`error`.
2. Linking integration updates lifecycle to `linking`, then `connected`.
3. Disconnect updates lifecycle to `disconnected`.
4. `integrationStore.list()` exposes lifecycle status/message for each integration.
5. Integrations dialog uses lifecycle status for visible gating and status text.

## Rollout
- Frontend-only change.
- No migration required; existing local storage remains valid.

## Rollback
- Revert this spec's companion code changes in `ui-intents`, `stores/integrations`, and `IntegrationsPanel`.
