# ESIG v1

ESIG is the compact binary signature sidecar for SDK reports. It is an SDK
artifact format, not an Edgerun internal wire, storage, cache, or local bridge
protocol.

ESIG v1 signs an ESRR segment report by hashing the exact report bytes and
signing:

```text
"edgerun-sdk.esig.v1.segment-report" || sha256(report.esrr)
```

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "ESIG"
4       2     signature_version = 1
6       2     abi_version = 2
8       4     flags
12      2     algorithm
14      2     public_key_len
16      2     signature_len
18      32    report_sha256
50      N     public_key
50+N    M     signature
```

Flags:

```text
bit 0: deterministic report binding
```

Algorithms:

```text
1 Ed25519
```

`verify-signed-segment-report` first checks the ESRR preflight bindings, then
checks that ESIG binds to the exact ESRR bytes and verifies the Ed25519
signature. This proves the signer endorsed the ESRR. When a third
`policy.espk` argument is supplied, the verifier also requires a matching
signer policy entry for the public key, segment id, segment hash, node role,
and capability.
