# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tstalwart-setup-smoke`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `stalwart-imap` | imap | 100 | 100 | 0 | 2246 | 44.52 | 19 | 25 | 26 | 346.50 MB / 57 |
| `stalwart-smtp` | smtp | 10 | 10 | 0 | 9553 | 1.05 | 305.821 | 381.778 | 381.778 | 325.00 MB / 57 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| stalwart | integrated_mail_server | 1 | stalwartlabs/stalwart:latest | image-provided | 0 | stalwart | smtp:25,imap:143,admin:8080 |

## Versions

### stalwart

```text
0.16.2
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| stalwart | 283215357 | 117 | 8396 |
