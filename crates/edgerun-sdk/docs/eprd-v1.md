# EPRD v1

EPRD is the SDK product catalog record.

On disk, product artifacts are rkyv archives of:

```text
SdkWireRecord::Product(ProductRecord)
```

The old compact byte format with an `EPRD` magic header has been removed. Do not
add a compatibility parser for it.

A product binds an app artifact graph, product id, pricing fields, developer
signature, and optional store signature.
