# EENT v1

EENT is the SDK entitlement record.

On disk, entitlement artifacts are rkyv archives of:

```text
SdkWireRecord::Entitlement(EntitlementRecord)
```

The old compact byte format with an `EENT` magic header has been removed. Do not
add a compatibility parser for it.

An entitlement binds an app, product, user, scope, validity window, and store
signature.
