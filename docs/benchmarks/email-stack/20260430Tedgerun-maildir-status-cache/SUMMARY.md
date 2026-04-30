# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tedgerun-maildir-status-cache`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `edgerun-imap` | imap | 100 | 100 | 0 | 3175 | 31.50 | 941 | 948 | 973 | 49.44 MB / 1 |
| `edgerun-smtp` | smtp | 10000 | 10000 | 0 | 67129 | 148.97 | 201.541 | 292.867 | 320.689 | 48.55 MB / 1 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| edgerun | integrated_reference_server | 1 | scratch | none | 30 | edgerun-server | smtp:25,imap:143 |

## Versions

### edgerun

```text
stack=edgerun
source_version=0.1.1
image_id=676e3ede657730f16b16718eade959ed29dc7b537ebebad649461f11a928a262 image_created=2026-04-30 05:15:55.93344644 +0000 UTC image_size=4902422
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| edgerun | 4902422 | 754 | 2201 |
