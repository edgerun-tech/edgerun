# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tedgerun-imap-buffered-nocache`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `edgerun-imap` | imap | 100 | 100 | 0 | 464 | 215.52 | 104 | 108 | 109 | 5.71 MB / 1 |
| `edgerun-smtp` | smtp | 100 | 100 | 0 | 644 | 155.20 | 190.681 | 201.522 | 202.080 | 4.33 MB / 1 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| edgerun | integrated_reference_server | 1 | scratch | none | 30 | edgerun-server | smtp:25,imap:143 |

## Versions

### edgerun

```text
stack=edgerun
source_version=0.1.1
image_id=aa2a0ee5b88f36e77aac40570d70aa2d3ddf86c47744cea3fd4b16f78ad8418a image_created=2026-04-30 05:12:42.308516467 +0000 UTC image_size=4902425
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| edgerun | 4902425 | 712 | 2198 |
