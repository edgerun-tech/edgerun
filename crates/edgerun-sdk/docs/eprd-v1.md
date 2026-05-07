# EPRD v1

EPRD is the compact binary product catalog artifact for paid apps and in-app
products. It is an SDK artifact format, not an Edgerun internal wire, storage,
cache, or local bridge protocol.

A product is signed by the developer first, then by the store. Entitlements can
bind to `sha256(product.eprd)` so purchases reference a concrete product offer
instead of only a string product id.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "EPRD"
4       2     product_version = 1
6       2     abi_version = 2
8       4     flags
12      2     product_kind
14      2     store_fee_bps
16      2     developer_share_bps
18      2     product_id_len
20      2     currency_len
22      8     price_minor
30      8     validity_seconds
38      32    app_id
70      32    developer_id
102     32    store_id
134     32    release_id = sha256(app.eapp)
166     32    terms_sha256
198     N     product_id
198+N   M     currency
...     2     developer_signature_len
...     S     developer Ed25519 signature
...     2     store_signature_len
...     T     store Ed25519 signature
```

Flags:

```text
bit 0: deterministic product body
```

Product kinds:

```text
1 license
2 subscription
3 consumable
4 feature
```

The developer signature covers the exact product body before
`developer_signature_len`:

```text
Ed25519(
  "edgerun-sdk.eprd.v1.developer-product" || sha256(eprd_body)
)
```

The store signature covers the product body plus developer signature:

```text
Ed25519(
  "edgerun-sdk.eprd.v1.store-product" || sha256(eprd_body_with_developer_signature)
)
```

Verification checks:

```text
app_id matches app.eapp
developer_id matches app.eapp developer_public_key
release_id matches sha256(app.eapp)
developer signature verifies with developer_id
store signature verifies with store_id
```
