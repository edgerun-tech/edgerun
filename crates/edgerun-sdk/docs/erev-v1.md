# EREV v1

EREV is the SDK revocation record.

On disk, revocation artifacts are rkyv archives of:

```text
SdkWireRecord::Revocation(Revocation)
```

The old compact byte format with an `EREV` magic header has been removed. Do not
add a compatibility parser for it.

A revocation binds a target kind, target hash, issuer, issued-at time, optional
reason, and issuer signature.
