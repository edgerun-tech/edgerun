# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tedgerun-status-sidecar-v2`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `edgerun-imap` | imap | 100 | 100 | 0 | 477 | 209.64 | 111 | 117 | 119 | 49.23 MB / 1 |
| `edgerun-smtp` | smtp | 10000 | 10000 | 0 | 63912 | 156.46 | 200.316 | 225.497 | 300.479 | 49.25 MB / 1 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| edgerun | integrated_reference_server | 1 | scratch | none | 30 | edgerun-server | smtp:25,imap:143 |

## Versions

### edgerun

```text
stack=edgerun
source_version=0.1.1
image_id=951892e4c2d53e73cbcc05578d8d463e89e5fdaafd52f0244eff98356164f70a image_created=2026-04-30 05:28:13.435841883 +0000 UTC image_size=4554266
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| edgerun | 4554266 | 885 | 12371 |
