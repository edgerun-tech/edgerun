# Exchange Boundary Audit

> Updated: 2026-05-04
> Scope: edgerun-exchange, edgerun-exchange-api, edgerun-exchange-worker, edgerun-node
> Status: provider-backed quote/order wiring exists. Exchange events have a codec boundary to the existing protocol event stream. The exchange API is stream-agnostic; the node owns the single event stream and is responsible for appending exchange events.

## Current Classification

### Real / Wired

| Component | Status | Notes |
|---|---|---|
| `ExchangeProvider` trait | Real | Provider boundary for quote, order creation, and status checks. |
| Provider adapters | Real-ish | SideShift, ChangeNOW, and feature-gated FF.io adapters exist. External API correctness still needs live integration testing. |
| `router::route_quote` | Real MVP | Filters providers, calls `quote()`, distinguishes no provider from provider failure, selects best by rate. Fees/health scoring are still TODO. |
| `ExchangeEvent` enum | Real model | Defines quote/order/status lifecycle facts. |
| `stream_codec` | Real boundary | Maps `ExchangeEvent` to existing wallet `EventType` values and protobuf payload bytes for `EventEnvelope.payload_object`. No new sink/storage path. |
| `edgerun-node/src/exchange_events.rs` | Node-owned stream bridge | Stores encoded exchange payloads as `OBJECT_KIND_PAYLOAD`, builds envelopes, appends/signs through `NodeStore`. |
| `projection::ExchangeOrderProjection` | Real projection | Pure derived order view over `ExchangeEvent` sequences. |
| `ExchangeStore` | Real in-memory store | Keeps fast quote/order state and records `ExchangeEvent`s. It does not own stream append. |
| `POST /v1/quote` | Provider-backed MVP | Calls provider routing, stores selected quote, returns public EdgeRun quote id, records `QuoteCreated` in the exchange store. |
| `POST /v1/order` | Provider-backed MVP | Uses stored quote/provider continuity and records `OrderCreated` plus initial `OrderStatusChanged` when needed. |
| `GET /v1/order/:id` | Event-projected in-memory view | Returns public order fields plus projected status metadata from `ExchangeStore`. |
| `GET /v1/assets` | Static catalog | Explicitly not provider truth. |
| `GET /health` | Honest degraded | Provider health checks are not implemented, so the API does not claim healthy. |

### Stub / Not Durable Yet

| Component | Status | Notes |
|---|---|---|
| Node route/bootstrap integration | Missing | Node-owned exchange append helper exists, but the node/server path still needs to drain exchange events and call it. |
| Worker poller | Stub | `run_poller()` is no-op and logs that no active polling occurs. |
| Worker reconciliation | Stub | `run_reconciliation()` is no-op and logs that no active reconciliation occurs. |
| Provider health checks | Missing | `/health` reports degraded until actual checks exist. |
| Audit persistence | Partial | Exchange lifecycle facts can be stream-backed by node-owned append helpers; raw provider responses are not yet separately stored. |
| Fee/health-aware routing | Partial | Routing currently scores by rate only. |

## Authority Boundary

The node owns exactly one event stream. Exchange does not own a stream runtime.

Correct flow:

```text
provider quote/order/status
  -> ExchangeEvent
  -> ExchangeStore records event for fast API projection
  -> node-owned wrapper drains/receives ExchangeEvent
  -> encode_exchange_event()
  -> WalletExchangeEventPayload bytes
  -> NodeStore::put_object(..., OBJECT_KIND_PAYLOAD, recipients)
  -> build_exchange_event_envelope(...)
  -> NodeStore::append_signed_event_blocking(...)
  -> decoded stream projection
```

No new event sink, log, stream runtime, or storage abstraction is introduced.

Until node/server integration calls the node-owned append helper, API endpoints remain in-memory MVP paths.

## Existing Stream Event Mapping

Exchange uses the existing wallet event range in `edgerun.v0.stream.EventType`:

| ExchangeEvent | EventEnvelope.event_type |
|---|---|
| `QuoteCreated` | `EVENT_TYPE_WALLET_QUOTE_CREATED` |
| `OrderCreated` | `EVENT_TYPE_WALLET_ORDER_CREATED` |
| `DepositObserved` | `EVENT_TYPE_WALLET_ORDER_STATUS_CHANGED` |
| `ProviderStatusObserved` | `EVENT_TYPE_WALLET_ORDER_STATUS_CHANGED` |
| `OrderStatusChanged` | `EVENT_TYPE_WALLET_ORDER_STATUS_CHANGED` |
| `OrderCompleted` | `EVENT_TYPE_WALLET_ORDER_STATUS_CHANGED` |
| `OrderFailed` | `EVENT_TYPE_WALLET_ORDER_STATUS_CHANGED` |
| `ManualReviewRequired` | `EVENT_TYPE_WALLET_ORDER_STATUS_CHANGED` |

The precise exchange lifecycle variant is carried inside `WalletExchangeEventPayload` as the payload object content.

## Projection Boundary

`edgerun-exchange/src/projection.rs` is the canonical projection layer for orders.

Rules:

1. Projection functions are pure over event sequences.
2. Projection code does not call providers.
3. Projection code does not mutate storage.
4. API handlers may combine projection results with public stored fields, but must not expose provider internals.
5. Durable state should reuse this projection over stream-decoded `ExchangeEvent` values.

`GET /v1/order/:id` returns derived fields:

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

## Remaining Required Work

1. Expose `exchange_events` from `edgerun-node` crate root or wire it from the node server module that already owns `NodeStore + signer + stream_id`.
2. Add a node-owned wrapper around exchange route handling that drains newly recorded `ExchangeEvent`s and calls `append_exchange_events_to_node_stream`.
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
cargo check -p edgerun-node
cargo check -p edgerun-exchange-worker
cargo test -p edgerun-exchange
cargo test -p edgerun-exchange-api
cargo test -p edgerun-node
cargo test -p edgerun-exchange-worker
```
