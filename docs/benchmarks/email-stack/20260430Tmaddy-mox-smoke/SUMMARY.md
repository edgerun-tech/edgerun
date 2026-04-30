# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tmaddy-mox-smoke`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `maddy-imap` | imap | 10 | 10 | 0 | 167 | 59.88 | 77 | 79 | 79 | 26.40 MB / 20 |
| `maddy-smtp` | smtp | 100 | 100 | 0 | 616 | 162.10 | 46.372 | 56.946 | 66.000 | 28.17 MB / 20 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| maddy | integrated_mail_server | 1 | foxcpp/maddy:latest | image-provided | 50 | maddy | smtp:25,imap:143 |
| mox | integrated_mail_server | 1 | r.xmox.nl/mox:latest | image-provided | 0 | mox | smtp:25,imap:143,admin:http/https |

## Versions

### maddy

```text
0.9.3 linux/amd64 go1.23.12
```

### mox

```text
v0.0.15-go1.24.2
linux/amd64
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| maddy | 58114574 | 256 | 2242 |
| mox | 47069261 | 138 | 2200 |
