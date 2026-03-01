# 2026-03-01 Cloudflare Domains, Tunnels, DNS, Access Usability v1

## Goal
Deliver a practical Cloudflare operations panel that lets users quickly inspect and act on four core areas from Intent UI:
1. Domains (zones)
2. Tunnels
3. DNS records
4. Access applications

## Non-goals
- No OAuth/browser-based Cloudflare login flow in this slice.
- No background syncing or long-running polling daemon.
- No automatic tunnel creation or destructive delete actions.
- No server-side persistence of Cloudflare API responses.

## Security and Constraint Requirements
- Keep token handling request-scoped and fail-closed for missing/invalid tokens.
- Do not log token values.
- Use loopback local bridge endpoints for Cloudflare API calls.
- Keep local bridge CORS behavior unchanged.
- Do not use port 8080.

## Design
- Extend node-manager local bridge with Cloudflare resource endpoints:
  - `GET /v1/local/cloudflare/zones`
  - `GET /v1/local/cloudflare/tunnels`
  - `GET /v1/local/cloudflare/access/apps`
  - `GET /v1/local/cloudflare/dns/records`
  - `POST /v1/local/cloudflare/dns/records/upsert`
- Build a new Cloudflare panel UI with:
  - token-aware loading state and clear call-to-action when token is absent,
  - zone selector for DNS and zone context,
  - tunnel and access app inventory cards,
  - DNS records table + quick upsert form for A/CNAME/TXT records.
- Keep Cloudflare integration verification flow unchanged and reuse the existing integration token.

## Acceptance Criteria
1. Cloudflare panel can load zones, tunnels, access apps, and DNS records via local bridge.
2. DNS upsert form can create/update a record and refresh list successfully.
3. Missing/invalid token states show actionable errors.
4. Cypress coverage validates key Cloudflare panel behavior with local bridge simulator.
5. Frontend checks and build pass.

## Rollout
- Ship node-manager endpoint additions and Cloudflare panel UI together.
- Validate with Cypress simulator coverage first, then operator token on local bridge runtime.

## Rollback
- Revert Cloudflare panel and local bridge endpoint changes tied to this spec.
