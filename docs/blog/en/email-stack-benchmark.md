---
title: Edgerun email benchmark: 10,000 local SMTP deliveries
date: 2026-04-30
author: Ken
summary: A first reproducible benchmark of Edgerun's all-in-one email server against Postfix, OpenSMTPD, and Exim.
tags: [email, benchmarks, release]
---
# Edgerun email benchmark: 10,000 local SMTP deliveries

Edgerun now runs the production mail stack for `edgerun.tech`: SMTP, SMTPS, submission, IMAP, IMAPS, DNS, HTTPS, webmail, DKIM signing, queue handling, this blog, and the Git host are served by one stripped `edgerun-server` binary.

Before making broader claims, I wanted a simple benchmark that avoids internet mail entirely. The test sends only loopback SMTP traffic to a local recipient, so it does not create spam, does not depend on remote providers, and can be reproduced one stack at a time.

## What was tested

Implemented in code today: Edgerun's SMTP server accepts local mail into a Maildir-backed mailbox and exposes the same mail through IMAP/webmail. For this benchmark, the isolated SMTP service was run on a high localhost port with relay disabled.

Host-only benchmark tooling: the comparison stacks are run in rootless Podman containers, one at a time, also on localhost high ports. The workload is 10,000 SMTP messages, 32 concurrent clients, 1 KiB body, one connection per message, local recipient only.

Out of scope for this pass: spam/content filtering, antivirus, outbound internet delivery, Stalwart's fair first-run account/domain setup, and full IMAP/webmail load testing.

## Results

<table>
<thead><tr><th>Stack</th><th>OK/Fail</th><th>Throughput</th><th>p95 latency</th><th>Peak memory during load</th></tr></thead>
<tbody>
<tr><td>Edgerun</td><td>10000/0</td><td>342.71 ops/s</td><td>140.577 ms</td><td>3.58 MB RSS</td></tr>
<tr><td>Postfix</td><td>10000/0</td><td>351.69 ops/s</td><td>141.674 ms</td><td>94.02 MB container memory</td></tr>
<tr><td>OpenSMTPD</td><td>9976/24</td><td>176.01 ops/s</td><td>227.632 ms</td><td>17.91 MB container memory</td></tr>
<tr><td>Exim</td><td>2146/7854</td><td>27.41 ops/s</td><td>1663.886 ms</td><td>219.9 MB container memory</td></tr>
</tbody>
</table>

The Postfix result is the closest throughput comparison. On this local-only SMTP accept workload, Edgerun is effectively tied with Postfix while using a much smaller memory footprint. Postfix is mature and fast, but it reaches that result with a larger multi-process service model. Edgerun stays in one process and peaks around 3.6 MB RSS in the isolated run.

OpenSMTPD is a useful smaller-MTA comparison. Its configuration is pleasant, but this 10k run had 24 failed operations and a much higher tail latency.

The Exim result should not be read as an Exim capacity claim. It is a configuration finding. The minimal rootless container setup accepted only 2,146 of 10,000 attempts while memory and process count climbed sharply. That target needs more tuning before it is a fair completed comparator.

## What surprised me

The first useful optimization was not a fancy queue. It was removing work that should not have been in the local benchmark path: loopback deliveries were spawning background SPF/DKIM/DMARC evaluation work. That made sense for real inbound remote mail, but not for localhost-only benchmark delivery. After skipping that evaluation for loopback peers, Edgerun moved much closer to Postfix.

I also tested a queue-backed local delivery wrapper. It was correct, but slower. The naive version wrote a durable spool file and then the worker immediately wrote the Maildir file, doubling disk work under load. That is not a release-worthy optimization. If we revisit it, it should be as a real batching design or an explicit durability mode, not as hidden extra I/O.

The current Edgerun hot path is deliberately conservative: write the Maildir message into `tmp/`, `sync_all()`, then atomically rename it into `new/`. That costs throughput, but it is a clear durability tradeoff and a better default for a reference server.

## Evidence

Raw evidence is committed with the source tree:

- Edgerun and Postfix sampled 10k run: `docs/benchmarks/email-stack/20260430Tscale-v2/`
- OpenSMTPD and Exim sampled 10k run: `docs/benchmarks/email-stack/20260430Tscale-v3/`
- First smoke-test evidence bundle: `docs/benchmarks/email-stack/20260430Tbench-v1/`

The current benchmark harness is host-only tooling. It uses the in-tree SMTP benchmark client and rootless Podman for comparison stacks. It binds only localhost ports and uses local-only recipients.

## What still needs testing

This is enough to publish a narrow claim: Edgerun's SMTP local-delivery path is already competitive with Postfix on a warmed 10k localhost workload while using far less memory.

It is not enough to claim victory over complete mail stacks. The next evidence should cover Stalwart with reproducible first-run configuration, IMAP mailbox operations, webmail inbox/send paths, outbound STARTTLS delivery, concurrent idle clients, and malformed or slow-client behavior.

That is the right shape for the project: make the claim small, publish the raw evidence, then keep tightening the server against the next workload.
