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
- The node owns identity-routed admission, local resource binding, and the
  signed local audit stream. Exchange code may produce `ExchangeEvent` values,
  but settlement authority should converge on admitted work and verifiable
  receipts; node-local audit events are replayable evidence and
  projections.
- Providers own external chain routing. Provider identifiers and order IDs are
  internal facts and are not public API.

## Settlement Flow

1. A marketplace app creates a `PaymentRequest` for the seller's desired
   settlement asset, amount, recipient address, expiry, and app/order context.
2. The app/user authorizes the settlement action as signed intent.
3. The intent and concrete wallet payload are carried by admitted work targeted
   at the settlement node identity.
4. The buyer chooses a pay asset and network.
5. Exchange routes a quote across configured providers and commission policy.
6. The buyer accepts the quote. Exchange creates a provider order and returns a
   deposit address plus the public EdgeRun order ID.
7. The buyer sends funds directly to the provider deposit address. EdgeRun never
   receives user crypto.
8. Provider status, chain observations, and derived status transitions are
   recorded as exchange audit events on the local node stream.
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

The host capability performs policy checks, calls exchange, requests admission
for cross-node work, and records signed local audit events through node-owned
stream code. Apps consume projections and receipts.
External HTTP endpoints can expose this capability to browsers, stores, and
remote clients, but HTTP is only ingress. Inside EdgeRun, settlement authority
comes from admitted work, signed intent, policy, and verifiable receipts.

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

Do not add a marketplace-specific byte codec. Do not let app code write stream
events directly. Do not treat HTTP request delivery, provider API success,
provider webhooks, chain polling, or local stream append as network authority
without admitted work and a verifiable receipt/proof trail.

## Near-Term Work

- Route exchange API ingress into admitted settlement work and node audit
  persistence instead of treating process-local memory as durable state.
- Add receipt projection APIs for marketplace apps.
- Move route scoring beyond rate-only selection to include fees, expiry,
  health, liquidity, amount bounds, and chain confidence.
- Normalize amount-side handling so buyer-pay and seller-settlement quotes are
  both first-class.
- Add app capability policy for marketplace settlement calls.
