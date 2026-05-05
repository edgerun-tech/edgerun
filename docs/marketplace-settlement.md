# Marketplace Settlement

EdgeRun Exchange is the non-custodial settlement layer for marketplace apps.
Marketplace growth comes from useful wasm apps, user stores, and app-store
distribution. Exchange earns commission by routing settlement, while user funds
move directly from the payer to provider-controlled route/deposit addresses and
then to the seller or app recipient. EdgeRun nodes never custody user crypto.

## Roles

- Marketplace apps own listings, carts, storefronts, fulfillment state, and
  buyer/seller UX.
- Exchange owns quotes, route selection, provider order creation, provider
  observations, canonical settlement status, and receipts.
- The node owns identity-routed command validation and the signed event stream.
  Exchange code may produce `ExchangeEvent` values, but authoritative settlement
  state is rebuilt from node-committed stream events.
- Providers own external chain routing. Provider identifiers and order IDs are
  internal facts and are not public API.

## Settlement Flow

1. A marketplace app creates a `PaymentRequest` for the seller's desired
   settlement asset, amount, recipient address, expiry, and app/order context.
2. The app/user authorizes the settlement action as an `AppIntent`.
3. The intent and concrete wallet payload are carried in a signed
   `CommandEnvelope` targeted at the settlement node identity.
4. The buyer chooses a pay asset and network.
5. Exchange routes a quote across configured providers and commission policy.
6. The buyer accepts the quote. Exchange creates a provider order and returns a
   deposit address plus the public EdgeRun order ID.
7. The buyer sends funds directly to the provider deposit address. EdgeRun never
   receives user crypto.
8. Provider status, chain observations, and derived status transitions are
   appended as exchange events on the node stream.
9. Completion emits a receipt payload that marketplace apps can verify without
   seeing provider internals.

## Wasm App Boundary

Wasm marketplace apps should not receive private keys, provider credentials, or
raw stream write access. They should call a narrow protocol-native host
capability:

```text
create_payment_request(request) -> payment_request_id
quote_payment(payment_request_id, pay_asset, amount_side) -> quote
open_settlement_order(quote_id, recipient_address, refund_address) -> order
get_settlement_status(order_id) -> projection
verify_receipt(receipt_id) -> receipt_projection
```

The host capability performs policy checks, calls exchange, and appends signed
events through node-owned stream code. Apps consume projections and receipts.
External HTTP endpoints can expose this capability to browsers, stores, and
remote clients, but HTTP is only ingress. Inside EdgeRun, settlement authority
comes from validated `CommandEnvelope` values, `AppIntent`, delegation policy,
and committed stream events.

## Commission Policy

Commission must be explicit policy, not hidden inside provider adapters:

- `edgerun_bps`
- optional minimum fee by settlement asset
- provider affiliate or partner IDs
- allowed pay/settlement asset pairs
- max slippage and quote expiry floor
- provider health and failure scoring
- user-visible total, fees, and expected settlement amount

Provider-specific affiliate details stay internal. Public quote responses expose
the amounts, expiry, and fee breakdown needed for informed consent.

## Protocol Facts

The internal wire boundary remains rkyv only. Settlement payloads stored as
protocol objects must be rkyv archives of concrete protocol types, such as
`WalletExchangeEventPayload`, `PaymentRequest`, and `Receipt`.

Do not add a marketplace-specific byte codec. Do not let app code write
authoritative stream events. Do not treat HTTP request delivery, provider API
success, provider webhooks, or chain polling as authority until the node records
signed exchange events.

## Near-Term Work

- Route exchange API ingress into protocol-native settlement commands and node
  stream persistence instead of process-local memory.
- Add receipt projection APIs for marketplace apps.
- Move route scoring beyond rate-only selection to include fees, expiry,
  health, liquidity, amount bounds, and chain confidence.
- Normalize amount-side handling so buyer-pay and seller-settlement quotes are
  both first-class.
- Add app capability policy for marketplace settlement calls.
