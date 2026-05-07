# EPAY v1

EPAY is the compact binary transfer artifact for payments between Edgerun
identities. It is an SDK artifact format, not an Edgerun internal wire,
storage, cache, or local bridge protocol.

The payment processor is deliberately not part of the authority model. EPAY
stores only `rail_kind` and `rail_ref_sha256`, so Stripe, ACH, crypto, card,
cash, or another rail can be used without changing app verification logic.

All integer fields are little-endian. Amounts are integer minor currency units
such as cents.

```text
offset  size  field
0       4     magic = "EPAY"
4       2     payment_version = 1
6       2     abi_version = 2
8       4     flags
12      2     purpose
14      2     currency_len
16      2     rail_kind_len
18      8     amount_minor
26      8     created_at
34      8     settled_at
42      32    app_id
74      32    release_id = sha256(app.eapp)
106     32    payer_id
138     32    payee_id
170     32    context_sha256
202     32    rail_ref_sha256
234     N     currency
234+N   M     rail_kind
...     2     payer_signature_len
...     S     payer Ed25519 signature
...     32    store_id, only when settled
...     2     store_signature_len
...     T     store Ed25519 signature, only when settled
```

Flags:

```text
bit 0: deterministic payment body
bit 1: settled by store/payment authority
```

Purposes:

```text
1 purchase
2 tip
3 reward
4 refund
5 payout
6 bounty
7 revenue_share
8 escrow_deposit
9 escrow_release
```

The payer signature covers the intent body with unset settlement fields:

```text
Ed25519(
  "edgerun-sdk.epay.v1.payer-intent" || sha256(epay_intent_body)
)
```

The store signature covers the settled body through `store_id`:

```text
Ed25519(
  "edgerun-sdk.epay.v1.store-settlement" || sha256(epay_settled_body)
)
```

Verification checks:

```text
app_id matches app.eapp
release_id matches sha256(app.eapp)
payer signature verifies with payer_id
if settled is required, store signature verifies with store_id
rail_ref_sha256 anchors external processor evidence without parsing it
```
