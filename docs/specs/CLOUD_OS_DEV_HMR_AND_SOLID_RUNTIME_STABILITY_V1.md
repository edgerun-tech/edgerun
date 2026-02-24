# Cloud OS Dev HMR and Solid Runtime Stability V1

## Goal and non-goals
- Goal:
  - Restore reliable dev-time hydration/HMR for `cloud-os` when served via Astro + Vite.
  - Eliminate Solid runtime warnings/errors caused by duplicate Solid resolution and rootless reactive state.
  - Keep onboarding/intent UI behavior functionally equivalent.
- Non-goals:
  - No redesign of onboarding or intent UX.
  - No protocol changes to MCP, router, or worker APIs.
  - No dependency swaps.

## Security and constraints
- Keep runtime deterministic and compatible with Bun-managed dependencies.
- Do not bind to port `8080`.
- Keep changes scoped to config/runtime glue (`astro.config.mjs`, component state setup) rather than broad refactors.

## Acceptance criteria
- `cloud-os` build succeeds after config/runtime changes.
- Dev config no longer depends on cross-origin websocket assumptions between browser port and Vite HMR backend.
- `IntentBar` no longer creates Solid computations outside a root at module initialization time.

## Rollout and rollback
- Rollout: apply Astro/Vite host/HMR + Solid dedupe config and move `IntentBar` global reactive state into a `createRoot` container.
- Rollback: revert `cloud-os/astro.config.mjs` and `cloud-os/src/components/IntentBar.tsx` if dev-time regressions appear.
