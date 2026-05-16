# ESRR v1

ESRR is the SDK segment execution report record.

On disk, `report.esrr` is a rkyv archive of:

```text
SdkWireRecord::SegmentReport(SegmentReportRecord)
```

The old compact byte format with an `ESRR` magic header has been removed. Do not
add a compatibility parser for it.

The report binds one node-local segment execution to the segment hash, parent
composition hash, public input lengths, output hash, deterministic cost, and
status. It is transport-neutral and does not expose private input bytes.
