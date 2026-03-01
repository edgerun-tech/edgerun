# 2026-03-01 Workflow Overlay Modularization V1

## Goal and non-goals
- Goal:
  - Reduce `WorkflowOverlay` complexity by extracting thread modeling and global event wiring into dedicated modules.
  - Preserve current behavior (no product/UX changes) while making future changes safer.
  - Improve reviewability and test stability by reducing single-file cognitive load.
- Non-goals:
  - Redesign conversations UX.
  - Change message schema, storage keys, or event names.
  - Refactor unrelated drawer/device functionality in this pass.

## Security and constraint requirements
- Preserve existing localStorage keys and event channels.
- Keep all cross-window/global event listeners paired with cleanup.
- Maintain deterministic thread ordering and selection fallback behavior.

## Acceptance criteria
1. Thread derivation/filter/sort/selection logic is extracted from `WorkflowOverlay.jsx` into a focused module.
2. Overlay global listeners (`super-v`, call-link readiness, pointer drag, `/` shortcut) are extracted into a focused module.
3. `WorkflowOverlay.jsx` composes extracted modules without changing runtime behavior.
4. Existing conversations Cypress behavior continues to pass.
5. Validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`
   - targeted Cypress conversations specs

## Rollout and rollback
- Rollout:
  - Add new layout modules and wire `WorkflowOverlay` to them.
  - Keep old behavior assertions via existing Cypress specs.
- Rollback:
  - Revert extracted modules and restore inlined logic in `WorkflowOverlay.jsx`.
