# ECMP v1

ECMP is the SDK composition manifest record for deterministic unit graphs.

On disk, `compose.edm` is a rkyv archive of:

```text
SdkWireRecord::Composition(CompositionRecord)
```

The old compact byte format with an `ECMP` magic header has been removed. Do not
add a compatibility parser for it.

`compose.edsl` is the editable source. `build-artifacts` validates the source
against the discovered units and writes the deterministic rkyv archive.
Execution uses byte copies between owned memories and never introduces shared
memory.
