# Email Stack 10k SMTP Comparator Evidence

Capture directory: `docs/benchmarks/email-stack/20260430Tscale-v3`

Host:

- Debian GNU/Linux 13
- Linux 6.12.74+deb13+1-amd64 x86_64
- Rootless Podman comparison containers, run one at a time as user `ken`

Workload:

- SMTP local-only delivery to `bench@example.test`
- 10,000 attempted messages
- 32 concurrent clients
- 1 KiB message body
- One SMTP connection per message
- No external delivery and no internet recipients
- Container memory sampled during load with `podman stats`

## Results

| Stack | Protocol | OK/Fail | Throughput | Avg Latency | p50 | p95 | p99 | Peak Memory During Load | Peak PIDs | Raw Evidence |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| OpenSMTPD | SMTP | 9976/24 | 176.01 ops/s | 155.28 ms | 115.178 ms | 227.632 ms | 1151.597 ms | 17.91 MB | 21 | `raw/opensmtpd-10k-sampled-smtp.out`, `raw/opensmtpd-10k-sampled-memory.tsv` |
| Exim | SMTP | 2146/7854 | 27.41 ops/s | 950.40 ms | 904.027 ms | 1663.886 ms | 2162.120 ms | 219.9 MB | 141 | `raw/exim-10k-sampled-smtp.out`, `raw/exim-10k-sampled-memory.tsv` |

## Cross-Run Context

The directly comparable Edgerun and Postfix sampled 10k results are in
`docs/benchmarks/email-stack/20260430Tscale-v2/`:

| Stack | OK/Fail | Throughput | p95 | Peak Memory During Load |
| --- | ---: | ---: | ---: | ---: |
| Edgerun isolated SMTP | 10000/0 | 342.71 ops/s | 140.577 ms | 3,580 kB RSS |
| Postfix | 10000/0 | 351.69 ops/s | 141.674 ms | 94.02 MB container memory |

## Notes

- OpenSMTPD remains a valid SMTP-only comparator, but the run had 24 failed
  operations and much higher tail latency than Edgerun/Postfix.
- Exim is not yet a fair completed 10k comparator with the current minimal
  container config. It accepted only 2,146 of 10,000 attempts while process
  count and memory climbed substantially. Treat this as configuration/tuning
  evidence, not as an Exim capacity claim.
- The comparison still needs a fair Stalwart SMTP/IMAP load test. The current
  evidence set only proves image/startup/bootstrap for Stalwart until domain,
  account, and anti-relay setup is automated reproducibly.
