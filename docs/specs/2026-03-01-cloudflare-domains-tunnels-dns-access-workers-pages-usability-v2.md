# 2026-03-01 Cloudflare Domains, Tunnels, DNS, Access, Workers, Pages Usability v2

## Goal
Extend the Cloudflare operations panel so operators can inspect six high-value resource classes from Intent UI:
1. Domains (zones)
2. Tunnels
3. DNS records
4. Access applications
5. Workers scripts
6. Pages projects

## Non-goals
- No worker deployment/publish workflow.
- No Pages deployment trigger workflow.
- No destructive delete actions for Workers/Pages.

## Security and Constraint Requirements
- Keep account API token request-scoped and never logged.
- Use local bridge loopback endpoints only.
- Keep fail-closed behavior on invalid/missing token/account context.
- Do not use port 8080.

## Design
- Add local bridge read endpoints:
  - `GET /v1/local/cloudflare/workers`
  - `GET /v1/local/cloudflare/pages`
- Reuse account-id resolution logic used by tunnels/access.
- Update Cloudflare panel UX:
  - keep active zone selector in top action bar,
  - remove duplicate zone selector from DNS section,
  - add Workers and Pages inventory sections.
- Update Cypress simulator to emulate workers/pages API responses.
- Update Cypress panel test to assert workers/pages rendering.

## Acceptance Criteria
1. Cloudflare panel shows workers and pages lists for valid token/account.
2. Top action bar has active domain selector used by DNS section.
3. DNS records still refresh for selected zone and upsert continues to work.
4. Cypress Cloudflare panel test validates workers/pages presence and DNS flow.
5. Frontend checks/build and relevant tests pass.

## Rollout
- Ship node-manager endpoints + panel/test updates together.
- No migration required.

## Rollback
- Revert endpoint and panel/test changes tied to this v2 document.
