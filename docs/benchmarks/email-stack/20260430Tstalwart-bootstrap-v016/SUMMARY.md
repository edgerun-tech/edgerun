# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tstalwart-bootstrap-v016`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |

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
| stalwart | 283215357 | 116 | 2214 |
