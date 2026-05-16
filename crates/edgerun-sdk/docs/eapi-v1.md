# EAPI v1

EAPI is the SDK function API manifest record.

On disk, `api.edm` is a rkyv archive of:

```text
SdkWireRecord::UnitApi(UnitApi)
```

The old compact byte format with an `EAPI` magic header has been removed. Do not
add a compatibility parser for it.

The verifier checks the archived function table against the typed runtime index
and the wasm binary's actual export signatures. `cost_base` and
`cost_per_byte` remain deterministic accounting fields used by the composition
executor.
