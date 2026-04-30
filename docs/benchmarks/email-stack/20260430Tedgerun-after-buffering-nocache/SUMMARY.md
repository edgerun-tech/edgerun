# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tedgerun-after-buffering-nocache`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `edgerun-imap` | imap | 100 | 100 | 0 | 3184 | 31.41 | 948 | 970 | 982 | 50.18 MB / 1 |
| `edgerun-smtp` | smtp | 10000 | 10000 | 0 | 64916 | 154.04 | 200.536 | 258.221 | 289.484 | 49.53 MB / 1 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| edgerun | integrated_reference_server | 1 | scratch | none | 30 | edgerun-server | smtp:25,imap:143 |

## Versions

### edgerun

```text
stack=edgerun
source_version=0.1.1
image_id=fb02aa6ee9656b5354e1db8c40c1e0332942011bbe15e47f7468e9f027d01c5b image_created=2026-04-30 05:13:19.848510523 +0000 UTC image_size=4902426
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| edgerun | 4902426 | 712 | 2205 |
