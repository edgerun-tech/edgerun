# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `edgerun-imap` | imap | 100 | 100 | 0 | 461 | 216.92 | 107 | 114 | 116 | 110.90 MB / 10 |
| `edgerun-smtp` | smtp | 10000 | 10000 | 0 | 17956 | 556.90 | 57.409 | 64.327 | 67.731 | 114.60 MB / 10 |
| `maddy-imap` | imap | 100 | 100 | 0 | 573 | 174.52 | 130 | 155 | 157 | 98.97 MB / 42 |
| `maddy-smtp` | smtp | 10000 | 10000 | 0 | 29302 | 341.27 | 90.809 | 125.659 | 150.601 | 148.50 MB / 42 |
| `postfix-smtp` | smtp | 10000 | 10000 | 0 | 28780 | 347.46 | 91.001 | 110.310 | 121.287 | 133.60 MB / 74 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| edgerun | integrated_reference_server | 1 | scratch | none | 30 | edgerun-server | smtp:25,imap:143 |
| maddy | integrated_mail_server | 1 | foxcpp/maddy:latest | image-provided | 50 | maddy | smtp:25,imap:143 |
| postfix | smtp_mta_only | 1 | debian:trixie-slim | postfix,ca-certificates | 11 | postfix | smtp:25 |

## Versions

### edgerun

```text
stack=edgerun
source_version=0.1.1
image_id=da7281e7e451e505b76774d854ce7801b985fe90d0e6facdcfc1a6d9a7aa5dc8 image_created=2026-04-30 06:24:21.789393541 +0000 UTC image_size=4812313
```

### maddy

```text
0.9.3 linux/amd64 go1.23.12
```

### postfix

```text
mail_version = 3.10.5
ca-certificates	20250419
postfix	3.10.5-1~deb13u1
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| edgerun | 4812313 | 818 | 2208 |
| maddy | 58114574 | 1360 | 2241 |
| postfix | 129522342 | 14360 | 2196 |
