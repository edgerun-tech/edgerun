# Rootless Edgerun/Postfix 10k Evidence Summary

This is a focused like-for-like rootless Podman evidence bundle for Edgerun and
Postfix. It replaces the earlier flawed native-Edgerun versus container-Postfix
comparison for this pair.

Claim class: like-for-like containerized performance evidence, with limits.
Both stacks ran as rootless Podman containers on the same host, one at a time,
using localhost-only high-port mappings.

Limits:

- single measured run, no warmup repetition;
- cached/local image build timing, not cold download/install timing;
- SMTP workload opens one connection per message;
- Postfix is SMTP-only, while Edgerun also exposes IMAP in the same container;
- not a final broad mail-stack comparison until OpenSMTPD, Exim, Dovecot, and
  Stalwart have comparable automated evidence.

## Environment

- Host: `Linux fw 7.0.2-1-cachyos x86_64`
- Container runtime: rootless Podman
- Podman storage: `/home/ken/.local/share/containers/storage`
- Evidence root: `docs/benchmarks/email-stack/20260430Trootless-edgerun-postfix-10k/raw`

## Stack Shape

| Stack | Container base | Components | Config lines | Image size |
| --- | --- | ---: | ---: | ---: |
| Edgerun | `scratch` | 1 | 30 | 4,886,040 bytes |
| Postfix | `debian:trixie-slim` | 1 | 11 | 129,522,342 bytes |

Edgerun image contents are one stripped static `edgerun-server` binary plus one
`server.yaml`. Postfix image contents are Debian plus `postfix` and
`ca-certificates`.

## SMTP 10k Result

| Stack | Requested | OK | Failed | Elapsed | Throughput | p50 | p95 | p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Edgerun | 10,000 | 10,000 | 0 | 76,883 ms | 130.07 ops/s | 232 ms | 245 ms | 250 ms |
| Postfix | 10,000 | 10,000 | 0 | 42,163 ms | 237.17 ops/s | 97 ms | 122 ms | 131 ms |

## IMAP Smoke

Only Edgerun had IMAP in this focused pair. The Dovecot comparison is collected
in separate runs.

| Stack | Requested | OK | Failed | Elapsed | Throughput | p50 | p95 | p99 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Edgerun | 100 | 100 | 0 | 3,165 ms | 31.60 ops/s | 942 ms | 951 ms | 978 ms |

## Memory During Load

Representative sampled values from `podman stats` during protocol load:

| Stack | Workload | Memory Range Seen | PIDs |
| --- | --- | --- | ---: |
| Edgerun | SMTP/IMAP | about 25-28 MB | 1 |
| Postfix | SMTP | about 113-116 MB | 74-75 |

## Immediate Reading

Postfix is faster on this one-message-per-connection SMTP workload in the same
rootless isolation mode. Edgerun is using much less memory and dramatically
fewer processes, but SMTP throughput and latency need optimization work before
we can claim competitive MTA performance.

The most likely Edgerun optimization areas are still queue-backed delivery,
connection/session handling, and reducing per-message synchronous filesystem
work. This bundle gives us a cleaner baseline for those changes.
