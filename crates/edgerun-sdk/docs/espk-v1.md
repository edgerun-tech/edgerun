# ESPK v1

ESPK is the SDK segment signer policy record.

On disk, `policy.espk` is a rkyv archive of:

```text
SdkWireRecord::SignerPolicy(SignerPolicy)
```

The old compact byte format with an `ESPK` magic header has been removed. Do not
add a compatibility parser for it.

An ESPK policy binds Ed25519 public keys to the exact segment boundary they are
allowed to sign for. This is local authorization input; it does not define
global trust roots.
