# ETRU v1

ETRU is the SDK trust policy record.

On disk, trust policy artifacts are rkyv archives of:

```text
SdkWireRecord::TrustPolicy(TrustPolicy)
```

The old compact byte format with an `ETRU` magic header has been removed. Do not
add a compatibility parser for it.

A trust policy is local verifier input. It binds roles, validity windows, public
keys, app ids, and developer ids; it does not install global trust roots.
