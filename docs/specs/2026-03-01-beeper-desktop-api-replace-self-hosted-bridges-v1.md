# 2026-03-01 Beeper Desktop API Replace Self-Hosted Bridges v1

## Goal

Replace self-hosted matrix bridge integration setup with a single Beeper Desktop API integration flow using a Beeper access token.

## Non-goals

- No auto-installation of Beeper Desktop.
- No support for starting matrix bridge MCP runtime containers in this slice.
- No change to GitHub MCP integration path.

## Security and Constraints

- Keep fail-closed verification for missing/invalid Beeper token.
- Keep token handling request-scoped for verification and vault-backed persistence behavior unchanged.
- Keep local bridge verification endpoint loopback-only.

## Acceptance Criteria

1. Messaging bridge providers are replaced by a single `beeper` integration provider in catalog/listing.
2. Integrations verification supports Beeper token via `/api/beeper/verify` with local fallback.
3. Caddy routes `/api/beeper/verify` to node-manager local bridge endpoint.
4. Integrations UI copy and tests reflect Beeper setup flow instead of matrix bridge token/runtime flow.
5. Node-manager and frontend validations pass.

## Rollout

- Add local Beeper verify endpoint in node-manager.
- Add Caddy route for Beeper verify.
- Replace bridge provider definitions and alias mapping.
- Update integration Cypress specs.

## Rollback

- Revert Beeper endpoint/routing/catalog/test changes in this slice.
