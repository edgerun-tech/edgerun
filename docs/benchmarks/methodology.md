# Edgerun Benchmarking Methodology

This document is the benchmark rulebook for Edgerun articles and release notes.
Every published benchmark should link here or state exactly where it deviates.

The goal is not to make Edgerun look good. The goal is to publish measurements
that another engineer can reproduce, criticize, and improve.

## Benchmark Classes

Every result must be labeled as one of these classes.

| Class | Valid Claim | Invalid Claim |
| --- | --- | --- |
| Like-for-like performance | Direct throughput, latency, memory, and CPU comparison | Any result where one stack is native and another is containerized |
| Deployment-mode comparison | What users experience when deploying each stack in its normal documented shape | Raw speed ranking across different isolation modes |
| Operator-experience evidence | Install time, config size, number of services, setup failures, logs, and required decisions | Runtime performance ranking |
| Production snapshot | What the live Edgerun deployment is doing now | Controlled benchmark comparison |
| Design/protocol note | Expected behavior or intended architecture | Implemented performance result |

If two stacks are not run under the same isolation, privilege, filesystem, and
networking model, the result is not a like-for-like performance benchmark.

## Isolation Rules

Do not compare native Edgerun against containerized Postfix, OpenSMTPD, Exim,
Dovecot, or Stalwart as a direct performance ranking.

For publishable throughput and latency claims, use one of these modes:

| Mode | Edgerun | Comparison Stacks | Valid Use |
| --- | --- | --- | --- |
| Native | Native process or systemd service | Native packages/services | Host service performance |
| Rootless container | Rootless Podman container | Rootless Podman container | Containerized deployment performance |
| Rootful container | Rootful container | Rootful container | Container runtime comparison |
| VM | Fresh VM image | Fresh VM image | Full deployment comparison |
| Production | Live Edgerun host | No direct comparator | Operational snapshot only |

When testing rootless Podman, all stacks must use the same rootless user, storage
driver, cgroup manager, CPU quota, memory limit, filesystem location, and
localhost-only port mapping strategy.

## Required Environment Capture

Each evidence bundle must include:

- host OS, kernel, architecture, CPU model, RAM, storage device/filesystem;
- whether the host is bare metal, VM, or container;
- Podman/container runtime version when containers are used;
- rootless/rootful status;
- cgroup manager and storage driver;
- git commit hash and dirty-tree status;
- exact binary paths and sizes;
- exact container image IDs and sizes;
- full generated benchmark config;
- command stdout/stderr for every setup, run, benchmark, and teardown step.

## Workload Rules

Mail benchmarks must avoid internet delivery unless the test is explicitly about
outbound delivery.

Default local SMTP workload:

- bind only localhost high ports;
- local recipient only, such as `bench@example.test`;
- no external relay;
- fixed message body size;
- fixed concurrency;
- one connection per message unless connection reuse is the stated workload;
- warmup run before measured run when comparing throughput;
- at least three measured repetitions for publishable claims.

Default IMAP workload:

- benchmark account created by the harness;
- fixed mailbox contents;
- fixed command sequence;
- separate login/list/select/logout, fetch, append, search, and idle workloads;
- report empty-mailbox and populated-mailbox results separately.

Timeouts and failures are data. Do not hide them. If a stack times out, publish
the timeout, logs, partial metrics, and resource samples.

Long benchmark runs must be intentional. The email evidence collector defaults
to short protocol caps and refuses protocol timeouts above five minutes unless
the run explicitly opts in with `EDGERUN_EMAIL_STORY_ALLOW_LONG_TIMEOUT=1`.

## Metrics To Publish

For performance results:

- requested operations;
- successful operations;
- failed operations;
- elapsed time;
- throughput;
- average, p50, p95, p99 latency;
- peak RSS or container memory during load;
- CPU sample during load;
- process/thread/PID count during load;
- disk bytes and filesystem notes when available;
- whether the run was cold, warm, or repeated.

For setup/operator results:

- cold image pull or package download time;
- build/install time;
- first successful listener readiness time;
- number of services/processes;
- number of config files;
- non-comment config line count;
- number of security-sensitive decisions;
- user/domain/tenant setup steps;
- whether the stack needs separate SMTP, IMAP, webmail, DNS, TLS, queue, or
  policy components;
- exact failure messages encountered during setup.

## Multitenancy Evidence

Multitenancy must be discussed as a separate dimension, not inferred from
single-user throughput.

For mail stacks, record:

- how domains are represented;
- how users/mailboxes are represented;
- whether per-domain policy is first-class or encoded through maps/config;
- how tenant isolation interacts with filesystem layout;
- how DKIM keys, TLS certificates, quotas, and relay policy are scoped;
- what must be restarted or reloaded when adding a tenant;
- whether setup is API-driven, file-driven, admin-UI-driven, or manual.

Do not claim a stack is better for multitenancy until the benchmark includes
multi-domain and multi-user setup evidence.

## Evidence Layout

Store evidence under:

```text
docs/benchmarks/<area>/<timestamp-or-name>/
```

Each bundle should contain:

```text
README.md
SUMMARY.md
raw/
```

`README.md` describes how the bundle was captured. `SUMMARY.md` may summarize
results, but raw output under `raw/` is authoritative.

The summary must state whether each item is:

- implemented in code;
- generated benchmark config;
- host-only tooling;
- container-only tooling;
- production snapshot;
- blocked by missing implementation;
- failed or timed out.

## Article Rules

Every benchmark article must separate:

- what was measured;
- how it was measured;
- what can be claimed;
- what cannot be claimed;
- what failed;
- what remains unmeasured.

Charts should visualize only comparable data. If a chart mixes deployment modes,
it must be titled as a deployment-mode chart, not a performance ranking.

Recommended wording:

- Good: "In this rootless Podman run, stack A accepted X ops/s and stack B
  accepted Y ops/s."
- Good: "In production, Edgerun's all-in-one process idles at X MB."
- Bad: "Edgerun is faster than Postfix" when Edgerun was native and Postfix was
  containerized.
- Bad: "Stalwart is slower" when first-run domain/account setup was not
  completed.

## Current Email Benchmark Implication

The earlier email evidence is useful, but its claims must be narrowed:

- Native Edgerun versus rootless container Postfix/OpenSMTPD/Exim is deployment
  evidence, not a direct performance ranking.
- The 10k SMTP numbers are still useful for finding Edgerun hot paths and
  comparator setup behavior.
- The next publishable comparison needs either all stacks native or all stacks
  containerized. The default email collector uses rootless Podman for Edgerun
  and the comparison stacks so those results can support like-for-like
  containerized claims.
- Operator-experience evidence should be collected separately and presented as
  a setup story, including install time, config decisions, failures, and
  multitenancy implications.
