# EDRR v1

EDRR is the SDK composition execution report record.

On disk, `report.edr` is a rkyv archive of:

```text
SdkWireRecord::ExecutionReport(ExecutionReportRecord)
```

The old compact byte format with an `EDRR` magic header has been removed. Do not
add a compatibility parser for it.

The report binds a concrete composition artifact, pinned components, public
input lengths, deterministic execution cost, output length, and output hash. It
does not include input bytes.
