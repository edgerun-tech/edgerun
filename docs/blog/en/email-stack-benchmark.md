---
title: Edgerun email benchmark: current state of the reference server
date: 2026-04-30
author: Ken
summary: A reproducible rootless Podman comparison of Edgerun, Maddy, and Postfix after the SMTP/IMAP runtime and Maildir indexing work.
tags: [email, benchmarks, release]
---
# Edgerun email benchmark: current state of the reference server

Edgerun now runs the production mail stack for `edgerun.tech`. SMTP, SMTPS,
submission, IMAP, IMAPS, DNS, HTTPS, webmail, DKIM signing, queue handling,
this blog, and the Git host are all served by one stripped `edgerun-server`
binary.

That is the product claim: a compact integrated server can replace a pile of
small daemons for a small deployment. The benchmark claim has to be narrower:
after the latest runtime and Maildir work, Edgerun is not merely small; it is
now faster than the measured Maddy and Postfix comparators on this local
delivery workload.

## What Changed

Implemented in code:

- the host runtime now uses a multi-threaded worker pool instead of running the benchmark service as a single host worker;
- SMTP and IMAP sessions use buffered transport I/O;
- local SMTP delivery accepts a recipient batch instead of repeating the same message work per recipient;
- IMAP mailbox status is maintained through a Maildir sidecar index instead of rescanning every message for each `SELECT`;
- SMTP delivery batches status-index updates and flushes them about every 500 ms;
- per-message Maildir file sync is now strict-mode opt-in through `EDGERUN_MAILDIR_SYNC_DELIVERY=1`.

The durability policy matters. The benchmarked default still writes each
message through Maildir `tmp/` and atomically renames into `new/`, but it no
longer pays a file `sync_all()` for every single message. Operators that want
that stricter delivery mode can turn it back on.

## What Was Tested

Host-only benchmark tooling: all stacks run one at a time in rootless Podman
containers on localhost high ports. The SMTP workload is 10,000 local messages,
32 concurrent clients, 1 KiB body, one connection per message, local recipient
only. The IMAP smoke workload logs in, lists mailboxes, selects the inbox, and
logs out.

Comparison set:

| Stack | Status in this evidence | Version captured |
| --- | --- | --- |
| Edgerun | measured SMTP and IMAP | `0.1.1` |
| Maddy | measured SMTP and IMAP | `0.9.3 linux/amd64 go1.23.12` |
| Postfix | measured SMTP | `3.10.5` |
| Mox | harness works in `localserve`, not ranked here | `v0.0.15` in smoke evidence |
| Stalwart | harness setup works, not ranked here | `0.16.2` in smoke evidence |
| Exim | dropped from active comparison | historical config failures only |

Out of scope for this pass: spam/content filtering, antivirus, outbound
internet delivery, realistic long-lived SMTP sessions, production-shape Mox
quickstart setup, production-tuned Stalwart policy, full webmail load, and
large IMAP fetch/search workloads.

## Five-Run Result

The freshest evidence is a five-repetition rootless Podman run:

`docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`

Important caveat: repetitions were run inside the same live container for each
stack, so mailbox/spool state grows across repetitions. This is a sustained
growing-state result, not five independent cold starts.

Median throughput across the five repetitions:

| Stack | SMTP median | IMAP median |
| --- | ---: | ---: |
| Edgerun | 527.03 ops/s | 216.92 ops/s |
| Maddy | 255.45 ops/s | 81.10 ops/s |
| Postfix | 126.10 ops/s | n/a |

Edgerun's SMTP throughput stayed stable across the repeated run:

| Stack | SMTP range across five runs | p95 latency pattern |
| --- | ---: | --- |
| Edgerun | 517.39-557.72 ops/s | about 64-67 ms |
| Maddy | 136.10-344.62 ops/s | degraded from 123.828 ms to 293.823 ms |
| Postfix | 111.86-347.70 ops/s | later runs reached about 1.09 s p95 |

The first repetition, before much state had accumulated, is also useful:

| Stack | SMTP throughput | SMTP p95 | Peak memory / PIDs |
| --- | ---: | ---: | --- |
| Edgerun | 557.72 ops/s | 64.173 ms | 113.20 MB / 10 |
| Maddy | 344.62 ops/s | 123.828 ms | 148.70 MB / 41 |
| Postfix | 347.70 ops/s | 110.324 ms | 134.20 MB / 75 |

That makes the narrow performance result clear: in this local one-message-per-
connection delivery workload, Edgerun is ahead on both first-run throughput and
sustained median throughput while keeping a much smaller process and image
footprint.

## Footprint

Runtime size and process count are still the most obvious operational
difference.

| Stack | Image size | Build time | Startup time | Process count during run |
| --- | ---: | ---: | ---: | ---: |
| Edgerun | 4,812,313 bytes | 797 ms | 2,199 ms | 10 |
| Maddy | 58,114,573 bytes | 1,310 ms | 2,230 ms | 41-45 |
| Postfix | 129,522,342 bytes | 14,198 ms | 2,186 ms | 75-76 |

Memory needs a more careful claim. Edgerun started lower than the comparators
in the SMTP run, but its sampled peak rose from 113.20 MB to 202.40 MB as the
five repetitions filled the mailbox. Maddy rose from 148.70 MB to 249.20 MB,
and Postfix rose from 134.20 MB to 209.00 MB. The current evidence supports a
strong image/process-footprint claim; it does not yet justify a broad
"lowest memory in every state" claim.

## What This Means

This benchmark used to say Edgerun was small and roughly competitive. That is
out of date.

The current evidence says something stronger but still bounded:

> Edgerun is a compact integrated mail stack worth considering for operators who want SMTP, IMAP, web, DNS, DKIM, queue handling, and site hosting in one small service. In the latest rootless Podman local-delivery benchmark, Edgerun delivered higher SMTP throughput than Maddy and Postfix, faster basic IMAP than Maddy, and a much smaller image and process footprint.

Avoid broader claims for now:

- not "Edgerun is faster than every mail server";
- not "Edgerun beats Dovecot on realistic IMAP";
- not "Edgerun beats Mox or Stalwart in production configuration";
- not "Exim/OpenSMTPD are slow" based on our earlier minimal harness issues.

## Evidence

Raw evidence is committed with the source tree:

- latest five-run Edgerun/Maddy/Postfix result: `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`
- single paired run after batching: `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched/`
- active smoke coverage for Edgerun, Postfix, OpenSMTPD, Dovecot, Maddy, Mox, and Stalwart: `docs/benchmarks/email-stack/20260430Tactive-smoke/`
- research status and comparator notes: `docs/benchmarks/email-stack/research-status-20260430.md`

The next work is to add independent cold-start repetitions, connection-reuse
SMTP, populated-mailbox IMAP fetch/search tests, disk I/O counters, CPU samples,
and production-shape Mox and Stalwart setup.
