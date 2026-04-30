# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tmaddy-10k`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `maddy-imap` | imap | 100 | 100 | 0 | 615 | 162.60 | 136 | 165 | 171 | 100.40 MB / 41 |
| `maddy-smtp` | smtp | 10000 | 10000 | 0 | 28679 | 348.68 | 88.814 | 122.395 | 147.094 | 142.10 MB / 41 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| maddy | integrated_mail_server | 1 | foxcpp/maddy:latest | image-provided | 50 | maddy | smtp:25,imap:143 |

## Versions

### maddy

```text
0.9.3 linux/amd64 go1.23.12
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| maddy | 58114574 | 240 | 2235 |
