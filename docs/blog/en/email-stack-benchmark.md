---
title: Edgerun email benchmark: current state of the reference server
date: 2026-04-30
author: Ken
summary: A reproducible comparison of Edgerun against Maddy and Postfix with current benchmark evidence and explicit limitations.
tags: [email, benchmarks, release, architecture]
---
# Edgerun email benchmark: current state of the reference server

Edgerun currently serves production mail for this deployment, while also exposing
the blog and code surfaces from the same runtime family. The benchmark claim is
therefore architectural as much as it is numeric: one stack is carrying multiple
roles and still remaining measurable.

This update is narrower than earlier notes:

- Edgerun now uses an updated SMTP/IMAP runtime in the reference server.
- benchmark outcomes are tracked in reproducible artifacts under `docs/benchmarks/`.
- claims are bounded by explicit workload limits and live-state test conditions.

## What changed in code

- Host runtime uses multi-threaded workers for benchmarked mail paths.
- buffered transport I/O in SMTP/IMAP sessions.
- recipient batching in local SMTP delivery.
- Maildir sidecar indexing for IMAP status.
- batched Maildir sync flushing cadence in the default path.
- index-driven status updates with strict sync mode opt-in.

These changes were made to remove avoidable per-message overhead in local delivery
workloads while keeping compatibility with production behavior.

## Workload shape and limitations

The five-run evidence is a local single-host comparison under rootless Podman:

- 10,000 messages
- 32 concurrent SMTP clients
- 1 KiB message bodies
- one connection per message
- local recipient-only delivery
- one repeatable local IMAP smoke path (LOGIN → LIST → SELECT → LOGOUT)

This is a practical but narrow slice. It is not internet-delivery stress,
long-lived session behavior, or a full webmail read pattern.

## Visualized benchmark statistics

<section class="bench-chart" aria-label="Benchmark medians">
<svg viewBox="0 0 860 360" xmlns="http://www.w3.org/2000/svg" role="img">
  <title>Median throughput and p95 latency</title>
  <rect width="860" height="360" fill="transparent"/>
  <text x="28" y="26" font-size="18" font-weight="700">Median SMTP and IMAP throughput</text>
  <text x="28" y="52" font-size="12" fill="#6a6f78">ops/s, five-run median</text>

  <g transform="translate(42,84)">
    <line x1="0" y1="230" x2="760" y2="230" stroke="currentColor" opacity="0.35"/>
    <g transform="translate(0,230)">
      <rect x="38" y="-190" width="74" height="190" fill="currentColor"/>
      <text x="75" y="8" text-anchor="middle" font-size="12">527</text>
      <text x="75" y="24" text-anchor="middle" fill="currentColor" font-size="11">Edgerun</text>

      <rect x="168" y="-124" width="74" height="124" fill="currentColor" opacity="0.78"/>
      <text x="205" y="8" text-anchor="middle" font-size="12">255</text>
      <text x="205" y="24" text-anchor="middle" fill="currentColor" font-size="11">Maddy</text>

      <rect x="298" y="-60" width="74" height="60" fill="currentColor" opacity="0.6"/>
      <text x="335" y="8" text-anchor="middle" font-size="12">126</text>
      <text x="335" y="24" text-anchor="middle" fill="currentColor" font-size="11">Postfix</text>
    </g>

    <g transform="translate(500,230)">
      <rect x="-84" y="-96" width="74" height="96" fill="currentColor" opacity="0.55"/>
      <text x="-47" y="8" text-anchor="middle" font-size="12">217</text>
      <text x="-47" y="24" text-anchor="middle" fill="currentColor" font-size="11">Edgerun IMAP</text>

      <rect x="48" y="-36" width="74" height="36" fill="currentColor" opacity="0.4"/>
      <text x="85" y="8" text-anchor="middle" font-size="12">81</text>
      <text x="85" y="24" text-anchor="middle" fill="currentColor" font-size="11">Maddy IMAP</text>
    </g>

    <text x="0" y="252" font-size="11">Stack</text>
    <text x="520" y="252" font-size="11">IMAP stacks</text>
  </g>
</svg>
</section>

<section class="bench-chart" aria-label="Process and image footprint">
<svg viewBox="0 0 860 320" xmlns="http://www.w3.org/2000/svg" role="img">
  <title>Process and image footprint</title>
  <text x="24" y="28" font-size="18" font-weight="700">Process footprint</text>
  <text x="24" y="50" font-size="12" fill="#6a6f78">image size and process count in warm-path run</text>

  <g transform="translate(40,90)">
    <line x1="0" y1="180" x2="760" y2="180" stroke="currentColor" opacity="0.35"/>
    <line x1="0" y1="0" x2="0" y2="180" stroke="currentColor" opacity="0.35"/>

    <g>
      <rect x="36" y="100" width="84" height="80" fill="currentColor"/>
      <text x="78" y="93" text-anchor="middle" font-size="12">4.8 MB</text>
      <text x="78" y="198" text-anchor="middle" font-size="11">Edgerun</text>

      <rect x="182" y="70" width="84" height="110" fill="currentColor" opacity="0.82"/>
      <text x="224" y="63" text-anchor="middle" font-size="12">58 MB</text>
      <text x="224" y="198" text-anchor="middle" font-size="11">Maddy</text>

      <rect x="328" y="48" width="84" height="132" fill="currentColor" opacity="0.65"/>
      <text x="370" y="41" text-anchor="middle" font-size="12">130 MB</text>
      <text x="370" y="198" text-anchor="middle" font-size="11">Postfix</text>
    </g>

    <text x="24" y="240" font-size="12" fill="currentColor" opacity="0.8">Image size is the strongest immediate operational delta in current evidence.</text>
  </g>
</svg>
</section>

## Comparison snapshot

| Stack | Status in this evidence | Version captured |
| --- | --- | --- |
| Edgerun | measured SMTP and IMAP | `0.1.1` |
| Maddy | measured SMTP and IMAP | `0.9.3 linux/amd64 go1.23.12` |
| Postfix | measured SMTP | `3.10.5` |
| Mox | harness works in `localserve`, not ranked here | `v0.0.15` |
| Stalwart | harness setup works, not ranked here | `0.16.2` |
| OpenSMTPD | smoke benchmark wired, not in ranked SMTP/IMAP table | `current harness config` |

## Why this is a bounded but meaningful achievement

The current evidence supports this:

- In this local one-message-per-connection workload, Edgerun’s SMTP throughput
  remains ahead of the measured comparators across five runs.
- IMAP behavior is materially better than Maddy in this smoke path.
- The integrated footprint is materially smaller.

That is a valid claim for this benchmark scope and version point. It is not a
general statement about all IMAP setups, internet routes, or production
multi-tenant workloads.

## Evidence

- latest five-run result: `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5/`
- paired single run after batching: `docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched/`
- active smoke coverage: `docs/benchmarks/email-stack/20260430Tactive-smoke/`
- research status: `docs/benchmarks/email-stack/research-status-20260430.md`

Next work is planned in production-hardened comparators and independent cold-start
repetitions with state reset per run.
