# ESIG v1

ESIG is the SDK segment report signature sidecar record.

On disk, `report.esig` is a rkyv archive of:

```text
SdkWireRecord::ArtifactSignature(ArtifactSignature)
```

The old compact byte format with an `ESIG` magic header has been removed. Do not
add a compatibility parser for it.

ESIG signs the exact report bytes through the domain-separated payload:

```text
"edgerun-sdk.esig.v1.segment-report" || sha256(report.esrr)
```
