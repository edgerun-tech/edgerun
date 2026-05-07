# ESET v1

ESET is the SDK settlement record.

On disk, settlement artifacts are rkyv archives of:

```text
SdkWireRecord::Settlement(SettlementRecord)
```

The old compact byte format with an `ESET` magic header has been removed. Do not
add a compatibility parser for it.

A settlement binds app/product context, entitlement hashes, gross/net amounts,
fees, and store signature material.
