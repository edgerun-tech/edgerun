# EDRR v1

EDRR is the compact binary execution report format for an ECMP run.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "EDRR"
4       2     report_version = 1
6       2     abi_version = 2
8       4     flags
12      2     input_count
14      2     component_count
16      2     step_count
18      2     composition_id_len
20      2     output_unit_id_len
22      8     cost
30      4     output_len
34      32    composition_sha256
66      32    output_sha256
98      N     composition_id utf-8 bytes
98+N    M     output_unit_id utf-8 bytes
...           input length records
...           component records
```

Each input length record:

```text
offset  size  field
0       4     input_len
```

Each component record:

```text
offset  size  field
0       2     unit_id_len
2       32    wasm_sha256
34      N     unit_id utf-8 bytes
```

The report binds a concrete ECMP artifact, its pinned components, the public
input lengths, deterministic execution cost, output length, and output hash.
It does not include input bytes.

`edgerun-sdk verify-report <report.edr>` recomputes the deterministic quote
from the public input lengths and rejects reports whose cost, output length,
composition hash, or component hashes do not match the discovered index.
