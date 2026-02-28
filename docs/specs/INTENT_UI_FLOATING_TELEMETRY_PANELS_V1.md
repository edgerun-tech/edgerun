# INTENT_UI_FLOATING_TELEMETRY_PANELS_V1

## Goal and Non-Goals

### Goal
- Provide reusable floating telemetry panels in Intent UI that are movable, resizable, and persist layout locally.
- Replace the ad-hoc Event Bus overlay with a reusable panel titled `EVENT BUS`.
- Add a second reusable panel titled `DOCKER LOGS` that renders live local bridge docker logs.

### Non-Goals
- No server-side persistence of panel layout.
- No new authentication model for local bridge log access.
- No rich log filtering/query UI in this iteration.

## Security and Constraints
- Keep local bridge fail-closed behavior unchanged.
- Docker logs are read only from the local bridge endpoint and exposed as plain text; no command execution from UI.
- Enforce bounded payloads (`tail_per_container`, total line limits) to avoid unbounded memory/UI growth.
- Follow existing frontend constraints: Bun-only workflow, no new heavy dependencies.

## Acceptance Criteria
- A reusable floating panel component exists and is used by at least two panels.
- Panel titles are uppercase.
- Panels support drag move and resize from UI.
- Position and size persist across reloads per panel ID.
- `EVENT BUS` panel shows newest-first lines, latest fully visible, transparent fade in list area.
- Default `EVENT BUS` list height is 150px and remains scrollable.
- `DOCKER LOGS` panel shows recent docker log lines from local bridge.

## Rollout and Rollback
- Rollout: ship component + `EVENT BUS` and `DOCKER LOGS` instances behind current Intent UI runtime.
- Rollback: remove panel instances from `App.jsx` and keep previous static overlay behavior.
