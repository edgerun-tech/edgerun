---
title: Email Stack Benchmark Baseline
date: 2026-04-30
author: Ken
summary: A draft benchmark plan and first production baseline for the Edgerun all-in-one email server.
tags: [email, benchmarks, release]
---

# Email Stack Benchmark Baseline

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

## Next Measurements

The next benchmark pass should add load scenarios:

- SMTP accept of small local messages.
- SMTP outbound delivery with STARTTLS.
- IMAP mailbox list and message fetch.
- Webmail inbox load and send.
- Concurrent idle connections.
- Malformed or slow client sessions to confirm bounded resource behavior.

The release claim should stay narrow until those numbers exist: Edgerun has an
excellent idle footprint for the implemented feature set, and the next work is
to prove the same discipline under load.
