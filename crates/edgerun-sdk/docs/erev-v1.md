# EREV v1

EREV is the compact binary revocation artifact for SDK verification. It is an
SDK artifact format, not an Edgerun internal wire, storage, cache, or local
bridge protocol.

The first version revokes exact keys or artifact hashes. Trusted verification
commands can receive one or more `.erev` files and fail when the verified
artifact or signing key is revoked.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "EREV"
4       2     revocation_version = 1
6       2     abi_version = 2
8       4     flags
12      2     revocation_kind
14      2     reason_len
16      8     issued_at
24      32    target hash or public key
56      32    issuer public key
88      N     reason bytes
88+N    2     signature_len
...     S     issuer Ed25519 signature
```

Flags:

```text
bit 0: deterministic revocation body
```

Revocation kinds:

```text
1 key
2 app/release
3 product
4 entitlement
5 payment
6 settlement
```

The signature covers the exact revocation body before `signature_len`:

```text
Ed25519(
  "edgerun-sdk.erev.v1.revocation" || sha256(erev_body)
)
```

Trusted verification behavior:

```text
key revocation blocks matching developer/store/payer keys
app revocation blocks sha256(app.eapp)
product revocation blocks sha256(product.eprd)
entitlement revocation blocks sha256(entitlement.eent)
payment revocation blocks sha256(payment.epay)
settlement revocation blocks sha256(settlement.eset)
```

A valid signature only proves who issued the revocation. Trusted verification
applies an EREV only when its issuer is also authorized by the local ETRU trust
policy for the revocation class, app scope, developer scope, and `issued_at`
time.

Issuer role mapping:

```text
key revocation: developer, app_store, product_store, entitlement_store,
                payment_store, or settlement_store
app revocation: developer or app_store
product revocation: developer or product_store
entitlement revocation: entitlement_store
payment revocation: payment_store or payer
settlement revocation: settlement_store
```

An otherwise well-formed EREV from an untrusted issuer is ignored by trusted
verification. This keeps revocation as a policy decision instead of treating any
supplied signed revocation file as a local root of authority.
