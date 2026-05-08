# EDUM v1

EDUM is the SDK unit manifest record.

On disk, `manifest.edm` is a rkyv archive of:

```text
SdkWireRecord::UnitManifest(UnitManifestRecord)
```

The old compact byte format with an `EDUM` magic header has been removed. Do not
add a compatibility parser for it.

The verifier checks ABI version, deterministic/owned-memory flags, import and
export counts, standard id, unit id, standard label, and the pinned wasm SHA-256.
