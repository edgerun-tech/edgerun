# EdgeRun Marketplace

EdgeRun Marketplace is the app and store distribution layer on top of EdgeRun
nodes. The first release should sell and install wasm apps, templates, and user
store packages while Exchange handles non-custodial crypto settlement. EdgeRun
earns from app/store sales, settlement routing, and marketplace fees without
custodying buyer, seller, or app funds.

## Release Shape

The first public release should be an app marketplace, not a general physical
goods marketplace. Apps, store themes, automations, and digital packages avoid
shipping, chargeback support, returns, fraud handling, and human dispute work.
User stores can come next once the same listing, install, payment, and receipt
machinery is proven.

Release 1 scope:

- browse signed app/store packages
- inspect publisher identity, version, required capabilities, price, and fee
  split
- buy with crypto through Exchange without EdgeRun custody
- install wasm packages onto a node after payment receipt verification
- let sellers publish packages and configure settlement addresses
- let EdgeRun collect marketplace fee through the same non-custodial settlement
  route

Out of scope for Release 1:

- custodial balances
- fiat checkout
- refunds and buyer arbitration
- physical delivery workflows
- escrow
- seller messaging
- ratings that require moderation

## Actors

- Buyer: chooses an app/package, approves payment, and installs it on a node.
- Publisher: signs app packages and receives seller settlement payouts.
- EdgeRun Marketplace: indexes packages, applies listing policy, presents
  checkout, and receives marketplace fee.
- Exchange: quotes and routes non-custodial settlement.
- Node: validates app intent, capability grants, install commands, and signed
  stream events.
- Provider: receives buyer crypto and pays the seller and fee recipients.

## Money Flow

The marketplace sale amount is split at settlement time:

```text
buyer wallet -> provider deposit address -> publisher settlement address
                                      \-> EdgeRun marketplace fee address
                                      \-> app/referral fee address, optional
```

EdgeRun never receives customer funds before seller payout. The buyer approves a
settlement intent that includes the package, price, settlement asset, expiry,
seller address, marketplace fee, optional app/referral fee, and provider route.
The node records committed settlement events and the marketplace marks the sale
paid from receipt/projection facts only.

If a provider cannot produce split payouts, Release 1 should either use a
provider affiliate/partner fee path or reject that route for marketplace
checkout. Do not settle through an EdgeRun wallet as a workaround.

## Trust Model

Marketplace browse data is advisory. Install and payment decisions are protocol
facts:

- app packages are signed by publisher identity
- listing records bind package object, publisher, price, settlement address, and
  required capabilities
- checkout creates a signed `AppIntent` over the exact listing and settlement
  terms
- settlement travels as an identity-routed `CommandEnvelope`
- node stream events are the authority for paid/installable state
- provider API responses and HTTP requests are observations only

## Core Objects

Use rkyv archives of concrete internal types at protocol boundaries. Until
these become generated protocol records, keep marketplace crate-local types
small and explicit.

```text
MarketplacePublisher
  publisher_id
  identity_ref
  display_name
  settlement_addresses
  signing_keys

MarketplacePackage
  package_id
  app_package_object
  publisher_id
  version
  content_hash
  required_capabilities
  screenshots_or_media_objects

MarketplaceListing
  listing_id
  package_id
  publisher_id
  price_asset
  price_amount
  seller_settlement_asset
  seller_settlement_address
  fee_policy
  status
  expires_at_ms

MarketplaceCheckout
  checkout_id
  listing_id
  buyer_node
  install_target_node
  payment_request_id
  quote_id
  order_id
  receipt_id
  status

MarketplaceInstallReceipt
  listing_id
  package_id
  receipt_id
  install_command
  installed_event
```

## Fee Policy

Fee policy must be displayed before payment and signed into checkout intent:

- seller gross amount
- EdgeRun marketplace basis points
- optional app/referral basis points
- provider/network fees
- buyer total
- seller net amount
- payout recipients

The node should reject checkout if the fee policy in the app intent differs
from the listing policy or exceeds local/user-approved limits.

## Capability Policy

Marketplace install is useful because buyers can see what an app wants before
paying or installing:

- storage scope
- network connect/bind scope
- device capability scope
- settlement capability scope
- update permission

Release 1 should require explicit approval for network, device, and settlement
capabilities. A paid app does not automatically get runtime authority; payment
unlocks installation eligibility, then node policy grants the minimum runtime
capabilities.

## User Flow

1. Buyer opens marketplace and browses listings.
2. Buyer selects package and sees publisher, version, price, fee split, and
   required capabilities.
3. Buyer chooses pay asset/network.
4. Marketplace creates a checkout and protocol-native payment request.
5. Exchange returns quote, provider deposit address, expiry, and fee disclosure.
6. Buyer sends crypto to provider deposit address.
7. Provider pays publisher and marketplace fee recipients.
8. Node records provider/chain observations and receipt events.
9. Marketplace verifies receipt projection.
10. Buyer installs package on selected node.
11. Node validates package signature, capability policy, and install command,
    then records install events.

## Publisher Flow

1. Publisher builds wasm app package.
2. Publisher signs package and metadata.
3. Publisher submits listing with price, settlement address, and required
   capabilities.
4. Marketplace validates package shape, signatures, content hash, and policy.
5. Listing becomes discoverable after index event.
6. Sales receipts and payout references are visible to publisher without
   exposing provider internals to public buyers.

## First Crate Boundary

Add a narrow `edgerun-marketplace` crate before building UI:

- marketplace domain types
- listing validation
- package/listing/checkouts projections
- fee policy validation
- receipt-to-install eligibility checks
- rkyv archive helpers for marketplace objects

Keep HTTP/browser API as ingress only. The crate should not own stream authority
or custody money. Node/store integration should append marketplace events and
derive projections from committed streams.

## Minimal Events

```text
MarketplacePublisherRegistered
MarketplacePackagePublished
MarketplaceListingPublished
MarketplaceListingSuspended
MarketplaceCheckoutCreated
MarketplaceCheckoutPaid
MarketplaceInstallAuthorized
MarketplacePackageInstalled
```

These events should eventually live in protocol-generated native records. For
the first crate, keep them as concrete rkyv marketplace types and archive them
through `edgerun-wire`.

## Release Plan

1. `edgerun-marketplace` domain crate with listing, package, checkout, fee
   policy, and projection tests.
2. Connect checkout to `edgerun-exchange` settlement intents and receipt
   verification.
3. Add package install eligibility: paid receipt plus package signature plus
   capability policy.
4. Add HTTP/browser ingress for browse, checkout, and install, routed into
   protocol commands.
5. Build a compact marketplace UI: browse, detail, checkout, receipt, install.
6. Seed first-party wasm packages and one seller flow.

## Release Gate

Do not release until these are true:

- every paid install can be proven from committed stream facts
- every checkout discloses and signs the fee split
- no route requires EdgeRun custody
- package install fails closed on signature or capability mismatch
- marketplace indexes can rebuild from stream events
- provider IDs remain internal except support/debug views
