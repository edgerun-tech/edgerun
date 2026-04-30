---
title: Email Stack Benchmark Baseline
date: 2026-04-30
author: Ken
summary: A draft benchmark plan and first production baseline for the Edgerun all-in-one email server.
tags: [email, benchmarks, release]
---

# Email Stack Benchmark Baseline

Methodology: follow `docs/benchmarks/methodology.md` for publishable benchmark
claims. Native Edgerun results must not be presented as direct throughput
rankings against containerized comparison stacks. Those runs are deployment-mode
or operator-experience evidence unless all stacks use the same isolation mode.
The rootless Podman harness now includes an Edgerun container path for
like-for-like containerized evidence.

Current focused rootless evidence:
`docs/benchmarks/email-stack/20260430Trootless-edgerun-postfix-10k/`.

This is the working draft for the public benchmark post. The goal is to compare
Edgerun against common self-hosted email stacks on the same host, with the same
measurement script, and with service scope stated explicitly.

## What Is Implemented

Implemented in code today: one `edgerun-server` process serves SMTP, SMTPS,
submission, IMAP, IMAPS, DNS, HTTP, HTTPS, webmail, mail queue handling, DKIM
signing, the blog host, and the Git host.

Host-only tooling: `scripts/benchmark-email-stack.sh` captures the service
memory, binary size, process accounting, and active listeners for Edgerun and
other installed mail stacks. It intentionally does not install packages or pull
external benchmark dependencies.

Host-only protocol benchmarking: `scripts/benchmark-email-protocol.sh` measures
SMTP local delivery throughput and latency, plus basic IMAP login/list/select
latency, using only loopback/local-recipient traffic. It does not send internet
mail and should be aimed at isolated benchmark ports, not public MX ports.

Currently out of scope for this benchmark: spam/content filtering and antivirus
pipelines. Those are important production features, but including them here
would make the comparison about filtering policy and databases rather than the
base mail server footprint.

## First Production Baseline

Snapshot taken on the production `mail` host after enabling stripped release
builds:

| Item | Value |
| --- | ---: |
| OS | Debian GNU/Linux 13 |
| Kernel | Linux 6.12.74+deb13+1-amd64 x86_64 |
| Binary | static-pie, stripped |
| `edgerun-server` size | 4,686,560 bytes |
| Idle RSS | 3,788 kB |
| Idle PSS | 3,784 kB |
| Private dirty | 520 kB |
| Swap | 0 kB |
| Long-running processes | 1 |

This is not the final optimized number. It is the first clean release baseline
after the TLS, queue, webmail, restart, and release-profile work.

## Comparison Set

The first fair comparison matrix should include:

| Stack | Scope to Measure |
| --- | --- |
| Edgerun | `edgerun-server.service`, all built-in listeners enabled |
| Postfix + Dovecot | MTA plus IMAP/LMTP service memory and listeners |
| Exim | MTA service memory and listeners |
| OpenSMTPD | MTA service memory and listeners |
| Stalwart | Stalwart mail server service memory and listeners |

Stalwart belongs in the benchmark because it is a modern integrated Rust mail
server, so it is a closer architectural comparison than traditional split MTA
plus IMAP stacks.

## Reproduction

Run the benchmark snapshot against the production host:

```sh
scripts/benchmark-email-stack.sh mail
```

Or run it locally on any comparison VM:

```sh
scripts/benchmark-email-stack.sh --local
```

For a publishable comparison, each stack should be measured on the same VM
class after boot, after service restart, and after a small warmup. The post
should publish raw output for each run and summarize only like-for-like service
scope.

Measure protocol throughput and latency without sending spam:

```sh
scripts/benchmark-email-protocol.sh smtp \
  --host 127.0.0.1 \
  --port 2525 \
  --count 1000 \
  --concurrency 8 \
  --from bench@example.test \
  --to bench@example.test \
  --message-bytes 1024
```

Measure IMAP command latency against a local benchmark account:

```sh
scripts/benchmark-email-protocol.sh imap \
  --host 127.0.0.1 \
  --port 1143 \
  --count 500 \
  --concurrency 4 \
  --user bench \
  --password bench
```

For competing stacks, installation/configuration difficulty should be recorded
as part of the result: packages needed, whether MTA packages conflict, how many
services must be configured, whether alternate localhost ports are easy to use,
how local-only delivery is enforced, how IMAP authentication is configured, and
how much configuration is needed before the benchmark can run without relaying
mail externally.

Use rootless Podman for the comparison stacks, one stack at a time:

```sh
scripts/benchmark-email-podman.sh build postfix
scripts/benchmark-email-podman.sh run postfix
scripts/benchmark-email-podman.sh bench-smtp postfix
scripts/benchmark-email-podman.sh stop postfix
```

The same runner supports `postfix`, `exim`, `opensmtpd`, `dovecot`, and
`stalwart`. It binds only localhost high ports by default: SMTP on `2525`, IMAP
on `1143`, and Stalwart admin on `18080`. This avoids production port conflicts
and avoids installing mutually conflicting MTA packages on the host.

The Stalwart target uses the current official container image documented by
Stalwart Labs: `stalwartlabs/stalwart:latest`. The older
`stalwartlabs/mail-server` image should not be used for new benchmark work.

Record configuration difficulty alongside throughput:

| Stack | Initial Difficulty | Configuration Notes |
| --- | --- | --- |
| Edgerun | Low | One YAML resource for SMTP and one for IMAP; local-only mode is `relay_enabled: false`. |
| Postfix | Medium | Easy package install, but host package conflicts through `mail-transport-agent`; comparable IMAP requires Dovecot. |
| Exim | Medium/high | Compact MTA, but minimal local-only config is less obvious; host package conflicts through `mail-transport-agent`. |
| OpenSMTPD | Low/medium | Small readable SMTP config; IMAP requires a separate service. |
| Dovecot | Medium | IMAP only; needs explicit Maildir, passdb, and userdb config for isolated benchmark accounts. |
| Stalwart | Medium | Integrated Rust mail server; container startup is easy, but fair benchmarking needs first-run domain/account/anti-relay setup. |

## Next Measurements

The next benchmark pass should add load scenarios beyond the current 10k SMTP
accept baseline:

- SMTP accept of small local messages.
- SMTP outbound delivery with STARTTLS.
- IMAP mailbox list and message fetch.
- Webmail inbox load and send.
- Concurrent idle connections.
- Malformed or slow client sessions to confirm bounded resource behavior.

The release claim should stay narrow until those numbers exist: Edgerun has an
excellent idle footprint for the implemented feature set, and the next work is
to prove the same discipline under load.

## Evidence References

The first reproducible evidence bundle is committed under
`docs/benchmarks/email-stack/20260430Tbench-v1/`.

The first warmed SMTP scale bundle is committed under
`docs/benchmarks/email-stack/20260430Tscale-v2/`. It uses 10,000 local-only SMTP
deliveries at concurrency 32 and samples memory during load. In that run,
Edgerun accepted 10,000/10,000 messages at 342.71 ops/s with 3,580 kB peak RSS;
Postfix accepted 10,000/10,000 at 351.69 ops/s with roughly 90 MB container
memory during load.

The follow-up comparator bundle is committed under
`docs/benchmarks/email-stack/20260430Tscale-v3/`. It adds sampled 10k SMTP runs
for OpenSMTPD and Exim. OpenSMTPD accepted 9,976/10,000 messages at 176.01
ops/s with 17.91 MB peak container memory. Exim accepted 2,146/10,000 attempts
with the current minimal config while peaking at 219.9 MB and 141 PIDs, so it is
tracked as a tuning/configuration finding rather than a fair completed result.

- Summary: `docs/benchmarks/email-stack/20260430Tbench-v1/SUMMARY.md`
- Scale summary: `docs/benchmarks/email-stack/20260430Tscale-v2/SUMMARY.md`
- Comparator summary: `docs/benchmarks/email-stack/20260430Tscale-v3/SUMMARY.md`
- Raw Edgerun production footprint:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/edgerun-live-stack.out`
- Raw Edgerun 10k SMTP result:
  `docs/benchmarks/email-stack/20260430Tscale-v2/raw/edgerun-10k-sampled-smtp.out`
- Raw Edgerun 10k memory samples:
  `docs/benchmarks/email-stack/20260430Tscale-v2/raw/edgerun-10k-sampled-memory.tsv`
- Raw Postfix 10k SMTP result:
  `docs/benchmarks/email-stack/20260430Tscale-v2/raw/postfix-10k-sampled-smtp.out`
- Raw Postfix 10k memory samples:
  `docs/benchmarks/email-stack/20260430Tscale-v2/raw/postfix-10k-sampled-memory.tsv`
- Raw OpenSMTPD 10k SMTP result:
  `docs/benchmarks/email-stack/20260430Tscale-v3/raw/opensmtpd-10k-sampled-smtp.out`
- Raw OpenSMTPD 10k memory samples:
  `docs/benchmarks/email-stack/20260430Tscale-v3/raw/opensmtpd-10k-sampled-memory.tsv`
- Raw Exim 10k SMTP result:
  `docs/benchmarks/email-stack/20260430Tscale-v3/raw/exim-10k-sampled-smtp.out`
- Raw Exim 10k memory samples:
  `docs/benchmarks/email-stack/20260430Tscale-v3/raw/exim-10k-sampled-memory.tsv`
- Raw Edgerun isolated SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/edgerun-smtp-only.out`
- Raw Postfix SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/postfix-smtp.out`
- Raw Exim SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/exim-smtp.out`
- Raw OpenSMTPD SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/opensmtpd-smtp.out`
- Raw Dovecot IMAP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/dovecot-imap.out`
- Raw Stalwart bootstrap/startup evidence:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/stalwart-run.out`
