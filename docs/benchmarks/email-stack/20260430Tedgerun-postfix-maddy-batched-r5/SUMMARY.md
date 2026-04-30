# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `edgerun-imap-r1` | imap | 100 | 100 | 0 | 476 | 210.08 | 110 | 118 | 121 | 201.80 MB / 10 |
| `edgerun-imap-r2` | imap | 100 | 100 | 0 | 466 | 214.59 | 108 | 118 | 119 | 200.40 MB / 10 |
| `edgerun-imap-r3` | imap | 100 | 100 | 0 | 459 | 217.86 | 106 | 111 | 114 | 198.70 MB / 10 |
| `edgerun-imap-r4` | imap | 100 | 100 | 0 | 461 | 216.92 | 108 | 113 | 116 | 197.20 MB / 10 |
| `edgerun-imap-r5` | imap | 100 | 100 | 0 | 459 | 217.86 | 106 | 114 | 115 | 194.50 MB / 10 |
| `edgerun-smtp-r1` | smtp | 10000 | 10000 | 0 | 17930 | 557.72 | 57.540 | 64.173 | 66.628 | 113.20 MB / 10 |
| `edgerun-smtp-r2` | smtp | 10000 | 10000 | 0 | 19327 | 517.39 | 61.309 | 66.891 | 70.344 | 132.90 MB / 10 |
| `edgerun-smtp-r3` | smtp | 10000 | 10000 | 0 | 18987 | 526.65 | 60.339 | 66.383 | 70.473 | 154.30 MB / 10 |
| `edgerun-smtp-r4` | smtp | 10000 | 10000 | 0 | 18932 | 528.19 | 60.256 | 65.511 | 67.529 | 178.10 MB / 10 |
| `edgerun-smtp-r5` | smtp | 10000 | 10000 | 0 | 18974 | 527.03 | 60.373 | 65.806 | 68.826 | 202.40 MB / 10 |
| `maddy-imap-r1` | imap | 100 | 100 | 0 | 1215 | 82.30 | 278 | 366 | 387 | 167.10 MB / 45 |
| `maddy-imap-r2` | imap | 100 | 100 | 0 | 1180 | 84.75 | 302 | 366 | 376 | 114.40 MB / 45 |
| `maddy-imap-r3` | imap | 100 | 100 | 0 | 1244 | 80.39 | 319 | 391 | 404 | 114.10 MB / 45 |
| `maddy-imap-r4` | imap | 100 | 100 | 0 | 1393 | 71.79 | 378 | 426 | 432 | 111.90 MB / 45 |
| `maddy-imap-r5` | imap | 100 | 100 | 0 | 1233 | 81.10 | 322 | 394 | 401 | 112.00 MB / 45 |
| `maddy-smtp-r1` | smtp | 10000 | 10000 | 0 | 29017 | 344.62 | 90.031 | 123.828 | 149.621 | 148.70 MB / 41 |
| `maddy-smtp-r2` | smtp | 10000 | 10000 | 0 | 31453 | 317.93 | 97.531 | 131.461 | 153.527 | 170.80 MB / 41 |
| `maddy-smtp-r3` | smtp | 10000 | 10000 | 0 | 39146 | 255.45 | 122.191 | 161.843 | 186.209 | 191.40 MB / 44 |
| `maddy-smtp-r4` | smtp | 10000 | 10000 | 0 | 55710 | 179.50 | 175.402 | 223.662 | 252.360 | 218.50 MB / 45 |
| `maddy-smtp-r5` | smtp | 10000 | 10000 | 0 | 73475 | 136.10 | 229.264 | 293.823 | 414.429 | 249.20 MB / 45 |
| `postfix-smtp-r1` | smtp | 10000 | 10000 | 0 | 28760 | 347.70 | 91.198 | 110.324 | 120.706 | 134.20 MB / 75 |
| `postfix-smtp-r2` | smtp | 10000 | 10000 | 0 | 28951 | 345.40 | 91.725 | 110.771 | 120.213 | 177.80 MB / 75 |
| `postfix-smtp-r3` | smtp | 10000 | 10000 | 0 | 79301 | 126.10 | 89.857 | 1084.636 | 1099.107 | 190.40 MB / 76 |
| `postfix-smtp-r4` | smtp | 10000 | 10000 | 0 | 89401 | 111.86 | 90.389 | 1087.931 | 1102.794 | 199.40 MB / 76 |
| `postfix-smtp-r5` | smtp | 10000 | 10000 | 0 | 88667 | 112.78 | 91.789 | 1089.474 | 1103.582 | 209.00 MB / 76 |

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
image_id=6dc24111b0e0a951db3b3751077bf18d706be2cfc27735b2c1b44f4973f2df00 image_created=2026-04-30 06:28:52.782149498 +0000 UTC image_size=4812313
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
| edgerun | 4812313 | 797 | 2199 |
| maddy | 58114573 | 1310 | 2230 |
| postfix | 129522342 | 14198 | 2186 |
