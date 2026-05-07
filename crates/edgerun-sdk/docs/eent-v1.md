# EENT v1

EENT is the compact binary entitlement artifact for paid apps and in-app
products. It is an SDK artifact format, not an Edgerun internal wire, storage,
cache, or local bridge protocol.

An entitlement is issued by a store for one app release, one subject, and one
product or feature. It carries the store/developer revenue split and embeds the
store signature.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "EENT"
4       2     entitlement_version = 1
6       2     abi_version = 2
8       4     flags
12      2     entitlement_kind
14      2     store_fee_bps
16      2     developer_share_bps
18      2     subject_id_len
20      2     product_id_len
22      2     purchase_id_len
24      8     valid_from
32      8     valid_until
40      32    app_id
72      32    developer_id
104     32    store_id
136     32    release_id = sha256(app.eapp)
168     32    terms_sha256
200     N     subject_id
200+N   M     product_id
...     P     purchase_id
...     2     signature_len
...     S     Ed25519 signature
```

Flags:

```text
bit 0: deterministic entitlement body
```

Entitlement kinds:

```text
1 license
2 subscription
3 consumable
4 feature
```

The signature covers the exact entitlement body before `signature_len`:

```text
Ed25519(
  "edgerun-sdk.eent.v1.store-entitlement" || sha256(eent_body)
)
```

Verification checks:

```text
app_id matches app.eapp
developer_id matches app.eapp developer_public_key
release_id matches sha256(app.eapp)
store signature verifies with store_id
optional requested subject/product filters match
```
