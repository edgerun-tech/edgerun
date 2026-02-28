# WWW/OS Worker Split V1

## Goal
- Separate `www.edgerun.tech` (marketing/docs) and `os.edgerun.tech` (control plane entry) into independent Cloudflare Workers.
- Keep a single shared frontend codebase/components in `frontend/`.

## Non-goals
- No new frontend app root.
- No component duplication/forking.
- No runtime auth redesign in this slice.

## Security and constraints
- `os.edgerun.tech` must fail-closed for non-control-plane paths.
- Worker deploy commands must stay pinned to explicit wrangler config files.
- Keep Bun-only workflow.

## Acceptance criteria
1. Two explicit wrangler configs exist:
   - `wrangler.jsonc` (`edgerun-www`)
   - `wrangler-os.jsonc` (`edgerun-os`)
2. Deploy scripts can deploy each target independently.
3. Target verification script validates distinct worker names/assets directories.
4. `os` worker serves control-plane routes and denies unrelated marketing paths.
5. Frontend checks/build pass with updated workflow.

## Rollout / rollback
- Rollout:
  1. Add `edgerun-os` worker config + worker entrypoint.
  2. Update target verification and deploy scripts.
  3. Deploy `edgerun-www`, then `edgerun-os`.
- Rollback:
  - Revert new `os` worker/config/script changes and point `os` hostname back to existing worker route.
