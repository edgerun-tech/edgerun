# Exchange Boundary Audit

> Updated: 2026-05-03
> Scope: edgerun-exchange, edgerun-exchange-api, edgerun-exchange-worker
> Status: provider-backed quote/order wiring exists. Order status is now projected from in-memory exchange events, but those events are still not durable protocol truth.

## Current Classification

### Real / Wired

| Component | Status | Notes |
|---|---|---|
| `ExchangeProvider` trait | Real | Provider boundary for quote, order creation, and status checks. |
| Provider adapters | Real-ish | SideShift, ChangeNOW, and feature-gated FF.io adapters exist. External API correctness still needs live integration testing. |
| `router::route_quote` | Real MVP | Filters providers, calls `quote()`, distinguishes no provider from provider failure, selects best by rate. Fees/health scoring are still TODO. |
| `ExchangeEvent` enum | Real model | Defines quote/order/status lifecycle events. Not yet persisted to core event stream. |
| `projection::ExchangeOrderProjection` | Real in-memory projection | Pure derived order view over `ExchangeEvent` sequences. |
| `ExchangeStore` | Real in-memory store | Keeps quotes, orders, and exchange events in memory. Not durable. |
| `POST /v1/quote` | Wired MVP | Calls provider routing, stores selected quote, returns public EdgeRun quote id. |
| `POST /v1/order` | Wired MVP | Uses stored quote, same selected provider, provider internal quote id, expiry check, and public EdgeRun order id. |
| `GET /v1/order/:id` | Event-projected MVP | Combines stored public order fields with derived status/event flags from `ExchangeOrderProjection`. |
| `GET /v1/assets` | Static catalog | Explicitly not provider truth. |
| `GET /health` | Honest degraded | Provider health checks are not implemented, so the API does not claim healthy. |

### Stub / Not Durable Yet

| Component | Status | Notes |
|---|---|---|
| Worker poller | Stub | `run_poller()` is no-op and logs that no active polling occurs. |
| Worker reconciliation | Stub | `run_reconciliation()` is no-op and logs that no active reconciliation occurs. |
| Event persistence | Missing | `ExchangeEvent` values are kept in memory only. They are not yet written to `edgerun-stream` / `NodeStore`. |
| Provider health checks | Missing | `/health` reports degraded until actual checks exist. |
| Audit persistence | Missing | Audit logger is logging-only / non-durable unless replaced by persistent storage. |
| Fee/health-aware routing | Partial | Routing currently scores by rate only. |

## Authority Boundary

Exchange state is **not yet authoritative protocol state**.

Current flow:

```text
provider quote -> ExchangeEvent::QuoteCreated in memory -> public exchange quote id
stored quote -> selected provider create_order -> ExchangeEvent::OrderCreated in memory -> projected order status
```

Target flow:

```text
provider quote -> ExchangeEvent::QuoteCreated -> append-only stream/object storage -> projected quote state
order creation -> ExchangeEvent::OrderCreated -> append-only stream/object storage -> projected order state
provider polling -> ProviderStatusObserved -> derived OrderStatusChanged -> projected order status
```

The in-memory store is an MVP event projection and should be treated as volatile.

## Projection Boundary

`edgerun-exchange/src/projection.rs` is the canonical in-memory projection layer for orders.

Rules:

1. Projection functions are pure over event sequences.
2. Projection code does not call providers.
3. Projection code does not mutate storage.
4. API handlers may combine projection results with public stored fields, but must not expose provider internals.
5. Durable future state should reuse this projection over persisted events.

`GET /v1/order/:id` now returns derived fields:

- `canonical_status`
- `event_count`
- `terminal`
- `manual_review_required`
- `latest_provider_status` if observed
- `latest_provider_status_detail` if observed
- `last_event_type`
- `updated_at_ms`

## Provider Isolation Rules

Provider internals must never appear in public API responses.

Internal-only fields:

- `provider_code`
- `provider_order_id`
- `quote_id_internal`

Public fields:

- EdgeRun quote id: `eq-...`
- EdgeRun order id: `ex-...`
- canonical asset ids
- canonical order status
- public deposit address
- public tx refs where applicable

## Quote → Order Continuity

Order creation must use the exact provider selected during quote creation.

Rules:

1. `store_quote()` persists:
   - public EdgeRun quote id
   - internal provider code
   - internal provider quote id
   - quote expiry
   - canonical asset/amount/rate fields
2. `handle_order()` validates the public quote id shape.
3. `handle_order()` rejects missing or expired quotes.
4. `handle_order()` finds the provider matching `stored_quote.provider_code`.
5. `handle_order()` calls `create_order()` with `stored_quote.quote_id_internal`.
6. `store_order()` refuses to store if the provider code does not match the stored quote provider.
7. Public response returns only the EdgeRun order id and public order fields.

## Event Model

All exchange state should eventually be derived from these immutable events:

1. `QuoteCreated`
2. `OrderCreated`
3. `DepositObserved`
4. `ProviderStatusObserved`
5. `OrderStatusChanged`
6. `OrderCompleted`
7. `OrderFailed`
8. `ManualReviewRequired`

State derivation rule:

```text
canonical order status = f(event sequence), not f(last provider status)
```

## Endpoint Classification

| Endpoint | Current Classification | Notes |
|---|---|---|
| `POST /v1/quote` | Provider-backed MVP | Requires providers initialized. Stores quote event in memory. |
| `POST /v1/order` | Provider-backed MVP | Uses stored quote provider continuity. Stores order event in memory. |
| `GET /v1/order/:id` | Event-projected in-memory view | Returns public order fields plus projected status metadata. |
| `GET /v1/assets` | Static catalog | Not provider truth. |
| `GET /health` | Degraded | Provider health checks pending. |

## Remaining Required Work

1. Persist `ExchangeEvent` into the core event/object model.
2. Rebuild exchange quote/order projections from persisted events.
3. Add poller that emits `ProviderStatusObserved`.
4. Add reconciliation that emits `ManualReviewRequired` on contradictions.
5. Add provider health checks.
6. Store raw provider responses as private/audit objects where policy allows.
7. Add fee/expiry/health-aware quote scoring.
8. Add live integration tests for SideShift/ChangeNOW behind ignored/env-gated tests.

## Verification Commands

```bash
cargo check -p edgerun-exchange
cargo check -p edgerun-exchange-api
cargo check -p edgerun-exchange-worker
cargo test -p edgerun-exchange
cargo test -p edgerun-exchange-api
cargo test -p edgerun-exchange-worker
```
