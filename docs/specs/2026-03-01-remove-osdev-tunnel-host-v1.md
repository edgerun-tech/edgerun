# 2026-03-01 Remove osdev Tunnel Host V1

## Goal and non-goals
- Goal:
  - Remove `osdev.edgerun.tech` as a configured tunnel hostname/default origin in repo-managed configuration.
  - Keep node-manager bridge and e2e tunnel behavior functional via remaining host mappings.
  - Ensure runtime defaults no longer implicitly depend on `osdev.edgerun.tech`.
- Non-goals:
  - Remove all Cloudflare tunnel support.
  - Change e2e hostname wiring (`osdeve2e.edgerun.tech`) in this pass.

## Security and constraint requirements
- Do not widen network exposure; only remove obsolete hostname references.
- Keep local bridge defaults deterministic and local-first where possible.
- Preserve docs accuracy for operator setup.

## Acceptance criteria
1. No source/runtime default references to `osdev.edgerun.tech` remain in active code/config docs.
2. `config/cloudflared/node-manager-tunnel.yml` no longer routes `osdev.edgerun.tech`.
3. Node manager OAuth redirect origin default no longer targets `osdev.edgerun.tech`.
4. Intent UI local bridge origin fallback no longer targets `osdev.edgerun.tech`.
5. Frontend validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Update tunnel config and docs to remove osdev hostname route.
  - Update default origin constants in node-manager and Intent UI bridge helper.
- Rollback:
  - Reintroduce `osdev.edgerun.tech` entries in tunnel config/docs and revert default origins.
