# Email Stack Benchmark Evidence Summary

Capture directory: `docs/benchmarks/email-stack/20260430Tbench-v1`

Host:

- Debian GNU/Linux 13
- Linux 6.12.74+deb13+1-amd64 x86_64
- Podman 5.4.2, rootless for comparison containers

Workload:

- SMTP: 50 local-only deliveries to `bench@example.test`, concurrency 4,
  localhost high port 2525 unless noted.
- IMAP: 50 login/list/select/logout sessions, concurrency 4, localhost high
  port 1143.
- Stalwart: startup/image/memory evidence only. It remains in bootstrap mode
  until first-run domain/account/anti-relay setup is automated.

## Results

| Stack | Protocol | OK/Fail | Throughput | Avg Latency | p95 | Memory Snapshot | Raw Evidence |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| Edgerun isolated SMTP | SMTP | 50/0 | 53.30 ops/s | 60.16 ms | 64 ms | RSS 2164 kB | `raw/edgerun-smtp-only.out`, `raw/edgerun-smtp-only-ps.out` |
| Edgerun production service | idle | n/a | n/a | n/a | n/a | PSS 4420 kB | `raw/edgerun-live-stack.out` |
| Postfix | SMTP | 50/0 | 48.92 ops/s | 64.58 ms | 102 ms | 2.572 MB | `raw/postfix-smtp.out`, `raw/postfix-stats.out` |
| Exim | SMTP | 50/0 | 10.58 ops/s | 321.52 ms | 384 ms | 6.308 MB | `raw/exim-smtp.out`, `raw/exim-stats.out` |
| OpenSMTPD | SMTP | 50/0 | 35.14 ops/s | 80.72 ms | 104 ms | 11.89 MB | `raw/opensmtpd-smtp.out`, `raw/opensmtpd-stats.out` |
| Dovecot | IMAP | 50/0 | 0.91 ops/s | 3821.82 ms | 4816 ms | 4.616 MB | `raw/dovecot-imap.out`, `raw/dovecot-stats.out` |
| Stalwart | bootstrap | n/a | n/a | n/a | n/a | 30.5 MB | `raw/stalwart-run.out`, `raw/stalwart-stats.out`, `raw/stalwart-logs.err` |

## Evidence Notes

- The Edgerun production snapshot is the live all-in-one server, not a
  container. It includes SMTP, SMTPS, submission, IMAP, IMAPS, DNS, HTTP,
  HTTPS, webmail, blog, and Git hosting.
- The Edgerun isolated SMTP run uses the same installed `/usr/local/bin`
  release binary on localhost port 2526 with `relay_enabled: false`.
- The comparison stacks are rootless Podman containers, one stack at a time.
  Their ports are bound only to `127.0.0.1`.
- Postfix, Exim, and OpenSMTPD are SMTP-only in this capture. A traditional
  full-stack comparison should pair Postfix/OpenSMTPD/Exim with Dovecot for
  IMAP.
- Stalwart is included as the modern integrated Rust comparison target, but this
  capture only records image/startup/bootstrap memory. Its fair protocol
  benchmark needs automated first-run configuration first.
