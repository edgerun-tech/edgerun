# EDUM v1

EDUM is the compact binary unit manifest format for `edgerun-sdk`.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "EDUM"
4       2     manifest_version = 1
6       2     abi_version = 2
8       4     flags
12      2     import_count
14      2     export_count
16      2     unit_id_len
18      2     standard_len
20      4     standard_id
24      32    wasm_sha256
56      N     unit_id utf-8 bytes
56+N    M     standard utf-8 bytes
```

Flags:

```text
bit 0: deterministic pure
bit 1: owns memory / no shared memory import
```

The executable API surface is verified from the wasm exports and the sibling
`api.edm` file. The binary manifest pins identity, hash, import/export counts,
ABI version, the numeric standard id exported by the wasm unit, and the unit's
standard label without JSON.
