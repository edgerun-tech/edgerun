# Intent UI EventBus WASM Worker V1

## Goal
- Run browser event bus execution in a dedicated worker thread.
- Back the worker with a compiled WebAssembly module from our Rust codebase.
- Keep current frontend APIs (`publishEvent`, `subscribeEvent`) stable.

## Non-goals
- No full replacement of backend edge-internal/edge-cluster transports in this slice.
- No cross-device sync protocol beyond current local snapshot ingestion.

## Security and constraints
- Fail-closed fallback: if wasm fails to load, worker must continue in JS mode and emit explicit runtime state.
- Event sequencing must be deterministic and monotonic per browser session.
- Keep builds deterministic and Bun-based for frontend workflows.

## Acceptance criteria
1. New `edgerun-event-bus-wasm` crate compiles to `wasm32-unknown-unknown`.
2. Intent UI build outputs `/intent-ui/eventbus.wasm` from that crate.
3. Browser event bus runs in `eventbus.worker.js` worker thread.
4. Event publish/subscribe flow routes through worker runtime state and timeline updates.
5. Clipboard/conversation events continue to publish successfully via event bus.
6. Frontend `check`, `build`, and targeted Cypress pass.

## Rollout / rollback
- Rollout:
  - `crates/edgerun-event-bus-wasm/**`
  - `Cargo.toml`
  - `frontend/scripts/build-intent-ui.mjs`
  - `frontend/intent-ui/src/workers/eventbus.worker.js`
  - `frontend/intent-ui/src/stores/eventbus.js`
  - `frontend/intent-ui/src/App.jsx`
  - `frontend/cypress/e2e/intent-ui-eventbus-wasm-worker.cy.js`
- Rollback: revert this change set to restore non-worker event bus behavior.
