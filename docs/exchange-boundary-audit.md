# Exchange Boundary Audit

> Generated: 2026-05-03
> Scope: edgerun-exchange, edgerun-exchange-api, edgerun-exchange-worker
> Status: compile-clean for all three crates

## What is Real

### Provider Adapters (edgerun-exchange)
| Component | Status | Notes |
|-----------|--------|-------|
| `ExchangeProvider` trait | REAL | Canonical trait with `quote()`, `create_order()`, `get_order_status()` |
| `SideShiftAdapter` | REAL | Full HTTP integration with sideshift.ai/api/v2 |
| `ChangeNOWAdapter` | REAL | Full HTTP integration with api.changenow.io/api/v2 |
| `FFioAdapter` | REAL (feature-flagged) | Full HTTP integration behind `ffio` feature |
| `ProviderCode` enum | REAL | Internal only, never exposed in public API |
| `ProviderFeatures` | REAL | Capability flags per provider |
| `ProviderContext` | REAL | HTTP client + timeout passed to providers |
| `ProviderQuote`, `ProviderOrder`, `ProviderStatus` | REAL | Canonical normalized types |
| `provider_mapping` | REAL | Maps provider status strings to canonical i32 codes |
| `status_machine` | REAL | Transition validation + contradiction detection |
| `ExchangeEvent` enum | REAL | 8 event types driving all state |
| `audit::AuditLogger` trait | REAL | Structured audit logging |
| `policy::RoutingPolicy` | REAL | Routing policy config |
| `router::route_quote` | REAL | Hard-filter → call providers → score → return best |
| `CanonicalOrderStatus` (proto) | REAL | 19-state enum in protobuf |
| `Quote`, `QuoteRequest`, `Order`, `TxRef`, `AssetRef` (proto) | REAL | Protobuf types |

### Worker (edgerun-exchange-worker)
| Component | Status | Notes |
|-----------|--------|-------|
| `poller` module | STUB | Declares config struct and `run_poller()` — no-op, logs stub warning |
| `reconciliation` module | STUB | Declares config struct and `run_reconciliation()` — no-op, logs stub warning |

### API Types (edgerun-exchange-api)
| Type | Status | Notes |
|------|--------|-------|
| `ApiQuoteRequest`, `ApiQuoteResponse` | REAL (type definitions) | Not yet wired to real routing |
| `ApiOrderRequest`, `ApiOrderResponse` | REAL (type definitions) | Not yet wired to real creation |
| `OrderStatus` enum | REAL | Public-facing, no provider names |
| `ApiAssetsResponse`, `AssetInfo` | REAL | Static catalog types |
| `ApiHealthResponse` | REAL | Reports degraded until real checks |

## What is Stub

| Component | Stub Detail |
|-----------|-------------|
| `route_quote` expiry filter | Does not filter expired quotes (no epoch-ms clock in no_std) |
| `route_quote` scoring | Sorts by rate only; fee weighting and health scoring TODO |
| Worker poller | No-op stub; logs "stub — no active polling" |
| Worker reconciliation | No-op stub; logs "stub — no active reconciliation" |
| Provider adapter HTTP paths | Real HTTP calls but no auth headers injected (API key not used in request) |
| `ProviderOrder.order_id` | Equals `provider_order_id` — no EdgeRun-generated public ID yet |
| Event stream persistence | Events are defined but not yet written to edgerun-stream |

## Provider Calls That Exist

| Provider | `quote()` | `create_order()` | `get_order_status()` |
|----------|-----------|-------------------|----------------------|
| SideShift | REAL — POST /api/v2/quotes | REAL — POST /api/v2/orders | REAL — GET /api/v2/orders/:id |
| ChangeNOW | REAL — POST /api/v2/exchange/estimated-amount | REAL — POST /api/v2/exchange/create | REAL — GET /api/v2/exchange/order/:id |
| FFio | REAL — POST /api/v1/quote | REAL — POST /api/v1/order | REAL — GET /api/v1/order/:id |

## What State is Durable

| State | Durability | Source |
|-------|-----------|--------|
| `ExchangeEvent` definitions | In-memory only | Defined in `events.rs`, not yet persisted |
| Quote response | In-memory (ProviderQuote) | Provider HTTP response |
| Order response | In-memory (ProviderOrder) | Provider HTTP response |
| Status | In-memory (ProviderStatus) | Provider HTTP response |
| Audit log entries | In-memory (SimpleAuditLogger) | Logged to edgerun-log, not persisted |

**TODO**: Wire ExchangeEvent into edgerun-stream for durable event storage.
**TODO**: Wire SimpleAuditLogger into edgerun-storage for persistent audit trail.

## Public API Endpoint Classification

| Endpoint | Classification | Current Behavior |
|----------|---------------|------------------|
| `POST /v1/quote` | STUB 501 | Parses/validates input, returns 501 "quote routing not yet implemented" |
| `POST /v1/order` | STUB 501 | Returns 501 "order creation not yet implemented" |
| `GET /v1/order/:id` | STUB 501 | Returns 501 "order status not yet implemented" |
| `GET /v1/assets` | STATIC CATALOG | Returns hardcoded asset list with `"source": "static_catalog"` and note |
| `GET /health` | DEGRADED (honest) | Returns `"status": "degraded"` with per-provider `"not_implemented"` |

## What Public API Promises Are Fake (Fixed)

| Issue | Status | Fix |
|-------|--------|-----|
| `/health` returned `"status": "healthy"` | FIXED | Now returns `"status": "degraded"` with reason |
| `/health` had empty providers map | FIXED | Now lists SIDESHIFT/CHANGENOW/FFIO as `"not_implemented"` |
| `/assets` implied provider truth | FIXED | Now includes `"source": "static_catalog"` and explanatory note |
| `run_server()` didn't actually run | FIXED | Renamed to `build_handler()` — returns handler, caller binds |
| `/v1/quote` said generic "not implemented" | FIXED | Now says "quote routing not yet implemented" |

## Event Model

All exchange state is derived from these immutable events:

1. **QuoteCreated** — quote object created with internal provider_code
2. **OrderCreated** — EdgeRun order_id, internal provider_order_id
3. **DepositObserved** — on-chain or provider-reported deposit
4. **ProviderStatusObserved** — observation only, not canonical
5. **OrderStatusChanged** — derived canonical transition with from/to
6. **OrderCompleted** — terminal with payout tx refs
7. **OrderFailed** — terminal with reason
8. **ManualReviewRequired** — provider contradiction or policy flag

State derivation rule: canonical order status = f(event sequence), NOT f(last provider status).

## Provider Isolation Rules

| Rule | Status |
|------|--------|
| `provider_code` is internal only | ENFORCED — `ProviderCode` not in public API types |
| `provider_order_id` is internal only | ENFORCED — proto field named `provider_order_id_internal` |
| Public order_id is EdgeRun-generated | TODO — currently equals provider_order_id |
| Provider raw responses stored as audit objects | TODO — raw JSON not yet persisted |
| No provider names in public API responses | ENFORCED — API types contain no provider fields |

## Status Machine

- 19 canonical statuses defined in protobuf
- Transition rules in `can_transition()` (wallet crate)
- Contradiction detection: terminal + provider disagreement → `OnHold`
- `InvalidStatusTransition` error now includes actual `from`/`to` status names

## Remaining Work

1. Wire ExchangeEvent into edgerun-stream for durable persistence
2. Wire quote/order/status API handlers to real provider routing
3. Generate EdgeRun order_id (not equal to provider_order_id)
4. Store raw provider responses as audit objects
5. Implement worker poller (periodic status polling)
6. Implement worker reconciliation (contradiction detection)
7. Add provider health checks to /health endpoint
8. Add fee-based scoring to route_quote
