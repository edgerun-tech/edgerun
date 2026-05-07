# ESET v1

ESET is the compact binary settlement statement for paid apps and in-app
payments. It is an SDK artifact format, not an Edgerun internal wire, storage,
cache, or local bridge protocol.

An ESET statement is signed by the store and binds a developer, period,
currency, aggregate amounts, and the exact entitlement artifacts included in
the settlement.

All integer fields are little-endian. Amounts are integer minor currency units
such as cents.

```text
offset  size  field
0       4     magic = "ESET"
4       2     settlement_version = 1
6       2     abi_version = 2
8       4     flags
12      2     entitlement_count
14      2     currency_len
16      8     period_start
24      8     period_end
32      8     gross_minor
40      8     processor_fee_minor
48      8     store_fee_minor
56      8     developer_net_minor
64      32    developer_id
96      32    store_id
128     N     currency
128+N   32*M  sha256(entitlement.eent) entries
...     2     signature_len
...     S     Ed25519 signature
```

Flags:

```text
bit 0: deterministic settlement body
```

The signature covers the exact settlement body before `signature_len`:

```text
Ed25519(
  "edgerun-sdk.eset.v1.store-settlement" || sha256(eset_body)
)
```

Verification checks:

```text
gross_minor == processor_fee_minor + store_fee_minor + developer_net_minor
store signature verifies with store_id
optional supplied entitlement files hash to the included entitlement hash set
optional supplied entitlement files have valid EENT store signatures
optional supplied entitlement files match developer_id and store_id
```
