# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tmox-localserve-smoke`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `mox-imap` | imap | 100 | 100 | 0 | 520 | 192.31 | 33 | 83 | 83 | 30.93 MB / 23 |
| `mox-smtp` | smtp | 100 | 100 | 0 | 8038 | 12.44 | 227.374 | 5051.271 | 5232.579 | 35.73 MB / 23 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| mox | integrated_localserve_mail_server | 1 | r.xmox.nl/mox:latest | image-provided | 9 | mox | smtp:1025,imap:1143,http:1080,https:1443 |

## Versions

### mox

```text
v0.0.15-go1.24.2
linux/amd64
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| mox | 47075876 | 186 | 2202 |
