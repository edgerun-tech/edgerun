# ESPK v1

ESPK is the compact binary signer policy format for SDK segment reports.

It is an SDK artifact format, not an Edgerun internal wire, storage, cache, or
local bridge protocol. Internal Edgerun boundaries remain rkyv-only.

An ESPK file binds Ed25519 public keys to the exact segment boundary they are
allowed to sign for. A verifier that receives an ESRR report and ESIG signature
can require an ESPK policy so a valid signature is not enough by itself.

## Layout

All integers are little-endian.

```text
offset  size  field
0       4     magic "ESPK"
4       2     version = 1
6       2     sdk abi version = 2
8       4     flags, bit 0 = deterministic
12      2     entry_count
...           entries
```

Each entry:

```text
offset  size  field
0       2     algorithm, 1 = Ed25519
2       2     public_key_len
4       2     segment_id_len
6       2     node_role_len
8       2     capability_len
10      32    segment_sha256
42      P     public key bytes
...     S     segment id utf-8 bytes
...     R     node role utf-8 bytes
...     C     capability utf-8 bytes
```

## Verification

`verify-signed-segment-report <report.esrr> <report.esig> <policy.espk>` checks:

- the ESRR preflight bindings against the discovered segment
- the ESIG Ed25519 signature over the exact ESRR byte hash
- an ESPK entry whose algorithm, public key, segment id, segment SHA-256, node
  role, and capability all match

This still does not define global trust roots. It is the local authorization
input that a node, app, or chain verifier can commit to.

