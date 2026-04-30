# Email Evidence Summary

Bundle: `docs/benchmarks/email-stack/20260430Tactive-smoke`

## Protocol Results

| Run | Mode | Requested | OK | Failed | Elapsed ms | Ops/s | p50 ms | p95 ms | p99 ms | Peak mem / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `dovecot-imap` | imap | 50 | 50 | 0 | 491 | 101.83 | 27 | 40 | 43 | 8.98 MB / 8 |
| `edgerun-imap` | imap | 50 | 50 | 0 | 1321 | 37.85 | 98 | 101 | 101 | 4.33 MB / 1 |
| `edgerun-smtp` | smtp | 100 | 100 | 0 | 2101 | 47.59 | 87.392 | 92.905 | 94.094 | 3.77 MB / 1 |
| `maddy-imap` | imap | 50 | 50 | 0 | 943 | 53.02 | 67 | 71 | 72 | 24.51 MB / 21 |
| `maddy-smtp` | smtp | 100 | 100 | 0 | 1185 | 84.37 | 46.769 | 50.576 | 56.828 | 26.92 MB / 21 |
| `mox-imap` | imap | 50 | 50 | 0 | 142 | 352.11 | 10 | 61 | 63 | 27.03 MB / 21 |
| `mox-smtp` | smtp | 50 | 50 | 0 | 1553 | 32.18 | 207.302 | 382.112 | 384.929 | 27.02 MB / 21 |
| `opensmtpd-smtp` | smtp | 100 | 100 | 0 | 5133 | 19.48 | 91.015 | 126.599 | 159.568 | 16.69 MB / 7 |
| `postfix-smtp` | smtp | 100 | 100 | 0 | 1183 | 84.46 | 45.813 | 49.788 | 66.945 | 19.38 MB / 16 |
| `stalwart-imap` | imap | 50 | 50 | 0 | 1056 | 47.35 | 18 | 20 | 20 | 334.00 MB / 57 |
| `stalwart-smtp` | smtp | 10 | 10 | 0 | 15565 | 0.64 | 409.979 | 5111.022 | 5111.022 | 327.50 MB / 57 |

## Stack Shape

| Stack | Shape | Components | Base | Packages | Config lines | Services | Ports |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| dovecot | imap_only | 1 | debian:trixie-slim | dovecot-core,dovecot-imapd,ca-certificates | 22 | dovecot | imap:143 |
| edgerun | integrated_reference_server | 1 | scratch | none | 30 | edgerun-server | smtp:25,imap:143 |
| maddy | integrated_mail_server | 1 | foxcpp/maddy:latest | image-provided | 50 | maddy | smtp:25,imap:143 |
| mox | integrated_localserve_mail_server | 1 | r.xmox.nl/mox:latest | image-provided | 9 | mox | smtp:1025,imap:1143,http:1080,https:1443 |
| opensmtpd | smtp_mta_only | 1 | debian:trixie-slim | opensmtpd,ca-certificates | 4 | smtpd | smtp:25 |
| postfix | smtp_mta_only | 1 | debian:trixie-slim | postfix,ca-certificates | 11 | postfix | smtp:25 |
| stalwart | integrated_mail_server | 1 | stalwartlabs/stalwart:latest | image-provided | 0 | stalwart | smtp:25,imap:143,admin:8080 |

## Versions

### dovecot

```text
dovecot_version=2.4.1-4 (7d8c0e5759)
ca-certificates	20250419
dovecot-core	1:2.4.1+dfsg1-6+deb13u4
dovecot-imapd	1:2.4.1+dfsg1-6+deb13u4
```

### edgerun

```text
stack=edgerun
source_version=0.1.1
image_id=4be0e7de0cdf05883aaabea7cd4799eee2e7efdaff50faf5aea7633fba532817 image_created=2026-04-30 03:56:43.481300327 +0000 UTC image_size=4902426
```

### maddy

```text
0.9.3 linux/amd64 go1.23.12
```

### mox

```text
v0.0.15-go1.24.2
linux/amd64
```

### opensmtpd

```text
version: OpenSMTPD 7.6.0-portable
ca-certificates	20250419
opensmtpd	7.6.0p1-1
```

### postfix

```text
mail_version = 3.10.5
ca-certificates	20250419
postfix	3.10.5-1~deb13u1
```

### stalwart

```text
0.16.2
```


## Images And Timing

| Stack | Image bytes | Build ms | Run ms |
| --- | ---: | ---: | ---: |
| dovecot | 142802767 | 142 | 2205 |
| edgerun | 4902426 | 394 | 2196 |
| maddy | 58114574 | 248 | 2240 |
| mox | 47075876 | 194 | 2196 |
| opensmtpd | 87110967 | 151 | 2182 |
| postfix | 129522342 | 148 | 2208 |
| stalwart | 283215357 | 109 | 13869 |
