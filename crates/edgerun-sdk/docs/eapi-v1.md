# EAPI v1

EAPI is the compact binary API manifest for an Edgerun wasm unit.

All integer fields are little-endian.

```text
offset  size  field
0       4     magic = "EAPI"
4       2     manifest_version = 1
6       2     abi_version = 2
8       2     function_count
10      ...   function records
```

Each function record:

```text
offset  size  field
0       2     name_len
2       1     param_count
3       1     result_count
4       2     flags
6       4     max_input_len
10      4     output_len
14      4     cost_base
18      4     cost_per_byte
22      N     name utf-8 bytes
22+N    P     param wasm value type bytes
22+N+P  R     result wasm value type bytes
```

Wasm value type bytes use the core wasm encoding:

```text
0x7f i32
0x7e i64
0x7d f32
0x7c f64
```

The verifier checks EAPI against both the typed runtime index and the wasm binary's
actual export signatures. This keeps the portable API surface outside Rust
without adding JSON or host wrappers.

`cost_base` and `cost_per_byte` are deterministic accounting fields. For a
function call, the ECMP executor charges:

```text
cost_base + cost_per_byte * metered_len
```

The metered length is selected by the function profile. Digest units meter the
input length argument. Parser/validator units meter the byte length argument.
`rfc2104_key_pad` meters the block size. `hmac_sha256` meters `key_len +
data_len`.
