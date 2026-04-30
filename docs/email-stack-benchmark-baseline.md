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
like-for-like containerized evidence. The latest focused bundle compares
Edgerun, Maddy, and Postfix under the same rootless harness after the
multi-threaded runtime, SMTP/IMAP buffering, Maildir status sidecar, and batched
status-flush work.

Current focused rootless evidence:
`docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`.

Current research status and recommended claim wording:
`docs/benchmarks/email-stack/research-status-20260430.md`.

Use `scripts/summarize-email-evidence.sh <bundle>` to render raw evidence into
reviewable Markdown tables without hand-copying individual output files.

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

Host-only protocol benchmarking: `edgerun-smtp-bench` is the preferred SMTP
load driver when available, and `scripts/benchmark-email-protocol.sh` remains
the portable shell fallback plus IMAP login/list/select driver. Both use only
loopback/local-recipient traffic. They do not send internet mail and should be
aimed at isolated benchmark ports, not public MX ports.

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
| OpenSMTPD | MTA service memory and listeners |
| Maddy | Integrated SMTP/IMAP service memory and listeners |
| Mox | Integrated SMTP/IMAP service memory and listeners after quickstart automation |
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
edgerun-smtp-bench \
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

The same runner currently supports `postfix`, `opensmtpd`, `dovecot`, `maddy`,
`mox`, and `stalwart`. It binds only localhost high ports by default: SMTP on
`2525`, IMAP on `1143`, and Stalwart admin on `18080`. This avoids production
port conflicts and avoids installing mutually conflicting MTA packages on the
host. Exim is intentionally removed from the active comparison set after the
minimal harness hit privilege/configuration issues before producing a fair
concurrency result.

The Stalwart target uses the current official container image documented by
Stalwart Labs: `stalwartlabs/stalwart:latest`. The runner bootstraps the v0.16
image through `stalwart-cli` before exposing local SMTP and IMAP. The older
`stalwartlabs/mail-server` image should not be used for new benchmark work.

The Maddy target uses the official `foxcpp/maddy:latest` image with an
implemented local-only benchmark config. The Mox target uses
`r.xmox.nl/mox:latest` in localserve mode for local SMTP/IMAP smoke evidence.
Mox localserve is useful for harness coverage, but production quickstart
domain/account setup still needs automation before fair local-only SMTP/IMAP
load tests.

Record configuration difficulty alongside throughput:

| Stack | Initial Difficulty | Configuration Notes |
| --- | --- | --- |
| Edgerun | Low | One YAML resource for SMTP and one for IMAP; local-only mode is `relay_enabled: false`. |
| Postfix | Medium | Easy package install, but host package conflicts through `mail-transport-agent`; comparable IMAP requires Dovecot. |
| OpenSMTPD | Low/medium | Small readable SMTP config; IMAP requires a separate service. |
| Dovecot | Medium | IMAP only; needs explicit Maildir, passdb, and userdb config for isolated benchmark accounts. |
| Maddy | Low/medium | Integrated SMTP/IMAP; the runner now has a local-only config and benchmark account. |
| Mox | Medium | Integrated SMTP/IMAP; localserve SMTP/IMAP works for development-mode evidence, but fair production benchmarking still needs quickstart domain/account automation. |
| Stalwart | Medium/high | Integrated Rust mail server; the runner now automates v0.16 bootstrap with `stalwart-cli`, but SMTP is only stable at low concurrency in the current local harness. |

## Current Focused Result

The freshest focused result is committed under
`docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`.
It runs five repetitions per stack inside one live rootless Podman container
per stack, so it is sustained/growing-state evidence rather than independent
cold-start evidence.

Median throughput across those five repetitions:

| Stack | SMTP median | IMAP median |
| --- | ---: | ---: |
| Edgerun | 527.03 ops/s | 216.92 ops/s |
| Maddy | 255.45 ops/s | 81.10 ops/s |
| Postfix | 126.10 ops/s | n/a |

Footprint from the same bundle:

| Stack | Image size | Process count during run |
| --- | ---: | ---: |
| Edgerun | 4,812,313 bytes | 10 |
| Maddy | 58,114,573 bytes | 41-45 |
| Postfix | 129,522,342 bytes | 75-76 |

This supports a narrow public claim: Edgerun is ahead of the measured Maddy and
Postfix comparators on this local one-message-per-connection SMTP delivery
workload, ahead of Maddy on the basic IMAP smoke workload, and much smaller in
image/process footprint. It does not prove broad mail-server superiority.

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
excellent idle footprint for the implemented feature set, and the current local
delivery numbers are strong, but production-shaped SMTP, IMAP, webmail, and
outbound delivery still need direct evidence.

## Evidence References

Start with the current research-status brief:
`docs/benchmarks/email-stack/research-status-20260430.md`. It records the
latest repeated Edgerun/Maddy/Postfix result, remaining-stack smoke coverage,
and safe public claim wording. The older scale bundles below are still useful,
but they should not be used to claim a direct native-versus-container speed
ranking.

The current focused repeated Edgerun/Maddy/Postfix bundle is committed under
`docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`.
It reports Edgerun SMTP median 527.03 ops/s, Maddy SMTP median 255.45 ops/s,
Postfix SMTP median 126.10 ops/s, Edgerun IMAP median 216.92 ops/s, and Maddy
IMAP median 81.10 ops/s. Treat this as sustained/growing-state evidence.

The current single paired Edgerun/Maddy/Postfix bundle is committed under
`docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched/`.

The older like-for-like Edgerun/Postfix rootless Podman bundle is committed
under
`docs/benchmarks/email-stack/20260430Trootless-edgerun-postfix-10k/`.
It is superseded for current ranking claims by the post-runtime/post-batching
Edgerun/Maddy/Postfix bundles.

The current remaining-stack rootless smoke evidence is committed under
`docs/benchmarks/email-stack/20260430Trootless-remaining-1k-smoke/`.

The current active all-stack smoke evidence is committed under
`docs/benchmarks/email-stack/20260430Tactive-smoke/`. It proves the cleaned
rootless harness can build, run, version-capture, and protocol-smoke Edgerun,
Postfix, OpenSMTPD, Dovecot, Maddy, Mox localserve, and Stalwart without Exim
in the active set. Treat it as harness-health evidence, not a ranking.

The current post-fix Edgerun-only evidence is committed under
`docs/benchmarks/email-stack/20260430Tedgerun-imap-buffered-nocache/` and
`docs/benchmarks/email-stack/20260430Tedgerun-status-sidecar-v2/`.
With Podman build cache disabled, buffered IMAP reads improved the small-mailbox
IMAP smoke path to 215.52 ops/s with p95 108 ms. After adding the
mutation-maintained Maildir status sidecar, the 10k-filled mailbox IMAP smoke path
completed 100/100 sessions at 209.64 ops/s with p95 117 ms. Use paired repeated
bundles, not these single-stack runs, for direct public rankings.

The current Maddy/Mox smoke evidence is committed under
`docs/benchmarks/email-stack/20260430Tmaddy-mox-smoke/`. Maddy has local-only
SMTP and IMAP smoke results; its Mox content is bootstrap/version evidence now
superseded by the localserve bundle.

The current Mox localserve smoke evidence is committed under
`docs/benchmarks/email-stack/20260430Tmox-localserve-smoke/`. Mox localserve
completed 100/100 SMTP messages at 12.44 ops/s and 100/100 IMAP smoke sessions
at 192.31 ops/s. This is development/test-mode evidence, not a production Mox
quickstart comparison.

The current Maddy 10k SMTP evidence is committed under
`docs/benchmarks/email-stack/20260430Tmaddy-10k/`. Maddy accepted
10,000/10,000 local-only SMTP messages at 348.68 ops/s and completed 100/100
IMAP smoke sessions at 162.60 ops/s.

The current Stalwart v0.16 setup-smoke evidence is committed under
`docs/benchmarks/email-stack/20260430Tstalwart-setup-smoke/`. The tested image
reports version 0.16.2. The harness bootstraps a local `example.test` domain
through `stalwart-cli` and completed 10/10 SMTP messages at concurrency 1 plus
100/100 IMAP smoke sessions. Treat this as setup/protocol coverage, not a
production-tuned throughput comparison.

The older Stalwart v0.16 bootstrap evidence is committed under
`docs/benchmarks/email-stack/20260430Tstalwart-bootstrap-v016/`. The tested
image reports version 0.16.2, starts in bootstrap mode, and exposes the HTTP
recovery/setup listener until first-run setup is completed.

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
tracked as a historical harness/configuration finding rather than a fair
completed result.

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
- Historical Exim 10k SMTP result, not part of the current active comparison:
  `docs/benchmarks/email-stack/20260430Tscale-v3/raw/exim-10k-sampled-smtp.out`
- Historical Exim 10k memory samples:
  `docs/benchmarks/email-stack/20260430Tscale-v3/raw/exim-10k-sampled-memory.tsv`
- Raw Edgerun isolated SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/edgerun-smtp-only.out`
- Raw Postfix SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/postfix-smtp.out`
- Historical Exim SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/exim-smtp.out`
- Raw OpenSMTPD SMTP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/opensmtpd-smtp.out`
- Raw Dovecot IMAP result:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/dovecot-imap.out`
- Raw Stalwart bootstrap/startup evidence:
  `docs/benchmarks/email-stack/20260430Tbench-v1/raw/stalwart-run.out`
