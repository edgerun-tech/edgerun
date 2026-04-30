# Email Stack 10k SMTP Scale Evidence

Capture directory: `docs/benchmarks/email-stack/20260430Tscale-v2`

Host:

- Debian GNU/Linux 13
- Linux 6.12.74+deb13+1-amd64 x86_64
- Edgerun installed as stripped release binaries under `/usr/local/bin`
- Postfix measured in a rootless Podman container on localhost high ports

Workload:

- SMTP local-only delivery to `bench@example.test`
- 10,000 messages
- 32 concurrent clients
- 1 KiB message body
- One SMTP connection per delivered message
- No external delivery and no internet recipients
- Memory sampled every 0.5 seconds during the benchmark run

## Results

| Stack | Protocol | OK/Fail | Throughput | Avg Latency | p50 | p95 | p99 | Peak Memory During Load | Raw Evidence |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Edgerun isolated SMTP | SMTP | 10000/0 | 342.71 ops/s | 93.27 ms | 87.270 ms | 140.577 ms | 175.062 ms | 3,580 kB RSS | `raw/edgerun-10k-sampled-smtp.out`, `raw/edgerun-10k-sampled-memory.tsv` |
| Postfix | SMTP | 10000/0 | 351.69 ops/s | 90.82 ms | 80.360 ms | 141.674 ms | 248.511 ms | ~94 MB container memory | `raw/postfix-10k-sampled-smtp.out`, `raw/postfix-10k-sampled-memory.tsv` |

## Notes

- Edgerun and Postfix are effectively tied on this local-only 10k SMTP accept
  workload after removing loopback-only background auth evaluation from the
  Edgerun benchmark path.
- Edgerun stays in one process and peaks at 3.6 MB RSS while accepting the
  10,000 messages.
- Postfix scales the same workload with many worker processes. The captured
  container memory sits around 90 MB during load and reports 73 processes in
  the final stats snapshot.
- The remaining Edgerun cost is likely the deliberate synchronous Maildir
  durability path: each accepted local recipient is written to `tmp/`, flushed
  with `sync_all()`, then atomically renamed into `new/`. That behavior is
  release-appropriate, but it is a clear optimization target if we add a
  documented durability mode or a batched local-delivery queue later.

## Evidence Files

- Edgerun SMTP metrics:
  `raw/edgerun-10k-sampled-smtp.out`
- Edgerun memory samples:
  `raw/edgerun-10k-sampled-memory.tsv`
- Edgerun process snapshot after run:
  `raw/edgerun-10k-sampled-ps-after.out`
- Postfix SMTP metrics:
  `raw/postfix-10k-sampled-smtp.out`
- Postfix memory samples:
  `raw/postfix-10k-sampled-memory.tsv`
- Postfix stats after run:
  `raw/postfix-10k-sampled-stats-after.out`
