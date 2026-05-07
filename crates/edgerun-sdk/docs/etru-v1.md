# ETRU v1

ETRU is the compact binary trust policy artifact for SDK verification. It is an
SDK artifact format, not an Edgerun internal wire, storage, cache, or local
bridge protocol.

Signature verification proves that a key signed an artifact. ETRU answers the
next question: whether that key is trusted for the role and scope being
verified.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "ETRU"
4       2     trust_policy_version = 1
6       2     abi_version = 2
8       4     flags
12      2     entry_count
14      ...   trust entries
```

Flags:

```text
bit 0: deterministic trust policy
```

Each trust entry:

```text
offset  size  field
0       2     role
2       2     reserved = 0
4       8     valid_from
12      8     valid_until
20      32    public_key
52      32    app_id scope, all zero for any app
84      32    developer_id scope, all zero for any developer
```

Roles:

```text
1 developer
2 app_store
3 product_store
4 entitlement_store
5 payment_store
6 settlement_store
7 payer
8 payee
```

Minimal policy matching:

```text
role matches
public_key matches
valid_from <= verification_time <= valid_until
app_id is zero or matches artifact app_id
developer_id is zero or matches artifact developer_id
```

The first implementation intentionally keeps ETRU unsigned. It is a local trust
root file supplied by the verifier. A later root policy can sign or delegate to
ETRU files once the revocation model exists.
