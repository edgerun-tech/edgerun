# Email Stack Research Status - 2026-04-30

This is the current evidence brief for deciding whether the Edgerun email stack
is worth considering, and what still needs to be measured before making broader
public claims.

## Current Position

Edgerun is already worth considering for small integrated deployments where the
operator values a compact, single-binary service surface. After the latest
runtime, SMTP/IMAP buffering, Maildir sidecar-index, and batched status-flush
work, the strongest current claim is now both operational and narrowly
performance-backed:

- implemented in code: one stripped `edgerun-server` can serve SMTP, SMTPS,
  submission, IMAP, IMAPS, DNS, HTTP, HTTPS, webmail, DKIM signing, queue
  handling, the blog, and the Git host;
- host-only evidence: production snapshot in
  `docs/email-stack-benchmark-baseline.md` measured a 4,686,560 byte static-pie
  binary and 3,788 kB idle RSS on the live host;
- like-for-like container evidence: in the latest five-repetition rootless
  Podman local-delivery run, Edgerun has higher median SMTP throughput than
  Maddy and Postfix, faster basic IMAP than Maddy, and a much smaller image and
  process footprint. Treat this as sustained growing-state evidence, because
  each stack's repetitions run inside one live container.

Do not claim that Edgerun is broadly faster than mature mail servers yet. The
latest rootless Edgerun/Maddy/Postfix bundle supersedes the older
native-vs-container and single-run rootless readings for direct local-delivery
claims, but it still does not cover realistic IMAP fetch/search, connection
reuse, outbound delivery, filtering, Mox production quickstart, or Stalwart
production policy.

## Local Evidence We Have

| Evidence | Class | What It Supports | Main Limit |
| --- | --- | --- | --- |
| `20260430Tedgerun-postfix-maddy-batched-r5/` | repeated like-for-like containerized performance evidence | Edgerun, Maddy, and Postfix under the same rootless Podman harness after runtime, Maildir sidecar, and batched status flush work | sustained/growing-state repetitions, not independent cold starts; Postfix SMTP-only |
| `20260430Tedgerun-postfix-maddy-batched/` | single like-for-like containerized performance evidence | Fresh paired Edgerun/Postfix/Maddy result after batching | single run only |
| `20260430Tactive-smoke/` | active harness health evidence | Edgerun, Postfix, OpenSMTPD, Dovecot, Maddy, Mox, and Stalwart all build/run and complete protocol smokes with versions captured | small smoke workload only, not throughput ranking |
| `20260430Tedgerun-imap-buffered-nocache/` | post-fix Edgerun smoke evidence | Forced-rebuild proof that buffered IMAP reads fix small-mailbox protocol overhead | smoke scale only, before 10k populated mailbox |
| `20260430Tedgerun-after-buffering-nocache/` | post-fix Edgerun 10k evidence | Forced-rebuild proof for buffered SMTP/IMAP under 10k SMTP delivery | populated IMAP still slow after mailbox fill |
| `20260430Tedgerun-maildir-status-cache/` | follow-up optimization evidence | Maildir status cache attempt did not materially improve populated IMAP smoke | documents remaining bottleneck, not a win |
| `20260430Tedgerun-status-sidecar-v2/` | post-fix Edgerun 10k evidence | Append-maintained Maildir status sidecar fixes populated-mailbox IMAP `SELECT` after 10k local deliveries | Edgerun-only run, still needs repeated paired comparison |
| `20260430Trootless-edgerun-postfix-10k/` | like-for-like containerized performance evidence | Edgerun versus Postfix under the same rootless Podman mode | single measured run, one connection per message, no warmup repetition |
| `20260430Trootless-remaining-1k-smoke/` | smoke/operator evidence | OpenSMTPD and Dovecot harness coverage, plus historical Exim failure evidence | 1k SMTP smoke for MTAs, not final 10k comparison |
| `20260430Tmaddy-10k/` | like-for-like containerized performance evidence | Maddy local-only SMTP/IMAP harness coverage at 10k SMTP scale | single measured run, no warmup repetition |
| `20260430Tmox-localserve-smoke/` | localserve smoke/operator evidence | Mox localserve SMTP/IMAP coverage with image/version capture | local development/test mode, not production quickstart |
| `20260430Tstalwart-setup-smoke/` | setup/protocol smoke evidence | Stalwart v0.16.2 automated setup plus SMTP/IMAP coverage | SMTP only stable at concurrency 1 in the current local harness |
| `20260430Tstalwart-bootstrap-v016/` | bootstrap/operator evidence | Stalwart v0.16.2 image/version/startup capture | superseded for protocol coverage by the setup smoke bundle |
| `20260430Tmaddy-mox-smoke/` | smoke/operator evidence | Initial Maddy smoke and Mox image/version bootstrap | superseded for Maddy/Mox protocol claims by newer bundles |
| `20260430Tscale-v2/` | mixed/development evidence | Edgerun optimization direction and older Postfix context | native/isolated Edgerun cannot be ranked directly against containerized stacks |
| `20260430Tscale-v3/` | comparator smoke/development evidence | OpenSMTPD and Exim failure/resource behavior under 10k load | Exim config is not a fair tuned capacity result |
| `20260430Tbench-v1/` | operator/bootstrap evidence | first footprint/setup notes for Postfix, Dovecot, OpenSMTPD, Exim, Stalwart | early run, no fair Stalwart SMTP/IMAP setup |

### Cleanest Current Rootless Results

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tedgerun-postfix-maddy-batched-r5`, the
current publishable local-delivery result is the five-repetition
Edgerun/Maddy/Postfix bundle:

| Stack | Workload | Median Throughput | Throughput Range | p95 Pattern | Peak Memory / PIDs |
| --- | --- | ---: | ---: | --- | ---: |
| Edgerun | SMTP 10k | 527.03 ops/s | 517.39-557.72 ops/s | about 64-67 ms | 113.20-202.40 MB / 10 |
| Maddy | SMTP 10k | 255.45 ops/s | 136.10-344.62 ops/s | 123.828-293.823 ms | 148.70-249.20 MB / 41-45 |
| Postfix | SMTP 10k | 126.10 ops/s | 111.86-347.70 ops/s | later runs about 1.09 s | 134.20-209.00 MB / 75-76 |
| Edgerun | IMAP smoke | 216.92 ops/s | 210.08-217.86 ops/s | 111-118 ms | 194.50-201.80 MB / 10 |
| Maddy | IMAP smoke | 81.10 ops/s | 71.79-84.75 ops/s | 366-426 ms | 111.90-167.10 MB / 45 |

Immediate reading: this is the first current bundle where Edgerun is ahead of
both Maddy and Postfix on the measured local SMTP delivery workload while also
retaining the smallest image and process footprint. Because the repetitions
share one live container per stack, label it as sustained/growing-state
evidence rather than independent cold-start repetition evidence. The memory
samples show Edgerun, Maddy, and Postfix all growing as state accumulates, so
the safest footprint claim is image size and process count, with memory reported
as measured rather than generalized.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tactive-smoke`, every active stack now has
a passing rootless Podman smoke result and version capture:

| Stack | Version | SMTP Smoke | IMAP Smoke | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: |
| Edgerun | 0.1.1 | 100/100, 47.59 ops/s | 50/50, 37.85 ops/s | 4.33 MB / 1 |
| Postfix | 3.10.5 | 100/100, 84.46 ops/s | n/a | 19.38 MB / 16 |
| OpenSMTPD | 7.6.0p1 | 100/100, 19.48 ops/s | n/a | 16.69 MB / 7 |
| Dovecot | 2.4.1 | n/a | 50/50, 101.83 ops/s | 8.98 MB / 8 |
| Maddy | 0.9.3 | 100/100, 84.37 ops/s | 50/50, 53.02 ops/s | 26.92 MB / 21 |
| Mox localserve | v0.0.15 | 50/50, 32.18 ops/s | 50/50, 352.11 ops/s | 27.03 MB / 21 |
| Stalwart | 0.16.2 | 10/10, 0.64 ops/s | 50/50, 47.35 ops/s | 334.00 MB / 57 |

Immediate reading: the active comparison harness is now working across all
non-Exim targets. This is not the benchmark result to publish as a ranking; it
is the evidence that the comparison set can be exercised reproducibly and that
version details are no longer missing.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tedgerun-imap-buffered-nocache`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Edgerun | SMTP smoke | 100/0 | 155.20 ops/s | 190.681 ms | 201.522 ms | 202.080 ms | 4.33 MB / 1 |
| Edgerun | IMAP smoke | 100/0 | 215.52 ops/s | 104 ms | 108 ms | 109 ms | 5.71 MB / 1 |

Immediate reading: buffered IMAP command reads fix the small-mailbox
login/list/select/logout benchmark. This bundle was built with Podman cache
disabled and a new image ID, so it is a clean rebuilt-image result.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tedgerun-after-buffering-nocache`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Edgerun | SMTP 10k | 10000/0 | 154.04 ops/s | 200.536 ms | 258.221 ms | 289.484 ms | 49.53 MB / 1 |
| Edgerun | IMAP after 10k SMTP fill | 100/0 | 31.41 ops/s | 948 ms | 970 ms | 982 ms | 50.18 MB / 1 |

Immediate reading: the SMTP hot-path cleanup still improves the focused
rootless Edgerun 10k accept run versus the earlier 130.07 ops/s bundle, but
the populated-mailbox IMAP result remains poor. The follow-up
`20260430Tedgerun-maildir-status-cache/` bundle shows that a simple
directory-mtime status cache did not materially change this path
(`31.50 ops/s`, p95 `948 ms`), so the remaining fix likely needs a real
mailbox index or append-maintained metadata rather than another session parser
tweak.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tedgerun-status-sidecar-v2`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Edgerun | SMTP 10k | 10000/0 | 156.46 ops/s | 200.316 ms | 225.497 ms | 300.479 ms | 49.25 MB / 1 |
| Edgerun | IMAP after 10k SMTP fill | 100/0 | 209.64 ops/s | 111 ms | 117 ms | 119 ms | 49.23 MB / 1 |

Immediate reading: the mutation-maintained Maildir status sidecar is the real
populated-mailbox IMAP fix. SMTP remains in the same broad range as the
post-buffering run, while IMAP after a 10k mailbox fill moves from roughly
31 ops/s and p95 near 970 ms to roughly 210 ops/s and p95 117 ms. This bundle
also fixed the harness issue that previously allowed a stale `edgerun-server`
binary to be reused: the Podman harness now builds the `edgerun-server` binary
target with the required `std smtp imap dns tls` features before copying it into
the scratch image.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Trootless-edgerun-postfix-10k`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Edgerun | SMTP 10k | 10000/0 | 130.07 ops/s | 232 ms | 245 ms | 250 ms | 44.43 MB / 1 |
| Postfix | SMTP 10k | 10000/0 | 237.17 ops/s | 97 ms | 122 ms | 131 ms | 120.10 MB / 76 |
| Edgerun | IMAP smoke | 100/0 | 31.60 ops/s | 942 ms | 951 ms | 978 ms | 28.06 MB / 1 |

Immediate reading: this older single-run result is superseded for current
performance ranking by `20260430Tedgerun-postfix-maddy-batched-r5/`. It remains
useful as historical evidence for why the runtime and Maildir indexing work was
needed.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Trootless-remaining-1k-smoke`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Dovecot | IMAP smoke | 100/0 | 151.06 ops/s | 106 ms | 186 ms | 201 ms | 14.18 MB / 10 |
| OpenSMTPD | SMTP 1k smoke | 985/0 | 12.48 ops/s | 93 ms | 5227 ms | 17240 ms | 25.89 MB / 21 |
| Exim | SMTP 1k smoke | 634/0 | 6.68 ops/s | 457 ms | 8416 ms | 13484 ms | 117.70 MB / 62 |

Immediate reading: Dovecot is a strong IMAP comparator and currently beats
Edgerun on the basic login/list/select/logout smoke. OpenSMTPD still needs
more harness/config work before its current smoke numbers should be treated as
performance claims. The Exim row is historical failure evidence and is removed
from the active comparison set.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tmaddy-10k`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Maddy | SMTP 10k | 10000/0 | 348.68 ops/s | 88.814 ms | 122.395 ms | 147.094 ms | 142.10 MB / 41 |
| Maddy | IMAP smoke | 100/0 | 162.60 ops/s | 136 ms | 165 ms | 171 ms | 100.40 MB / 41 |

Immediate reading: Maddy is now a working integrated SMTP/IMAP comparator in
the rootless harness. This older single-stack Maddy run is superseded for direct
ranking by the repeated Edgerun/Maddy/Postfix bundle, where Edgerun is ahead on
median SMTP throughput and basic IMAP smoke. The tested Maddy image reports
`0.9.3 linux/amd64 go1.23.12`. Mox is captured for image/version/bootstrap
evidence in `20260430Tmaddy-mox-smoke/`; the tested image reports
`v0.0.15-go1.24.2 linux/amd64`.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tmox-localserve-smoke`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Mox localserve | SMTP smoke | 100/0 | 12.44 ops/s | 227.374 ms | 5051.271 ms | 5232.579 ms | 35.73 MB / 23 |
| Mox localserve | IMAP smoke | 100/0 | 192.31 ops/s | 33 ms | 83 ms | 83 ms | 30.93 MB / 23 |

Immediate reading: Mox localserve is now a working SMTP/IMAP harness target.
Its SMTP path is not a fair production throughput comparison yet: localserve
does DNS, reputation, and content checks even in this local mode, and a
concurrency-32 probe hit Mox's per-IP connection limit. Treat the result as
evidence that the runner can exercise Mox and that production quickstart
automation is the next step.

From `./scripts/summarize-email-evidence.sh
docs/benchmarks/email-stack/20260430Tstalwart-setup-smoke`:

| Stack | Workload | OK/Fail | Throughput | p50 | p95 | p99 | Peak Memory / PIDs |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Stalwart | SMTP smoke | 10/0 | 1.05 ops/s | 305.821 ms | 381.778 ms | 381.778 ms | 325.00 MB / 57 |
| Stalwart | IMAP smoke | 100/0 | 44.52 ops/s | 19 ms | 25 ms | 26 ms | 346.50 MB / 57 |

Immediate reading: Stalwart is now a working SMTP/IMAP harness target. The
runner uses `stalwart-cli` v1.0.4 to convert the v0.16.2 bootstrap container
into a local `example.test` server with one benchmark account, plain IMAP on
port 143, and clear-text IMAP auth for loopback-only testing. SMTP is not yet a
fair throughput comparison: concurrency 2 produced a connection-close sample in
manual smoke testing, while concurrency 1 completed cleanly but slowly. Treat
this as setup and protocol coverage, not a production-tuned Stalwart result.

## External Research Context

Use primary project documentation as the baseline for comparisons:

- Postfix documents performance tuning as process, concurrency, queue, disk, and
  content-filter tuning work, and its architecture explicitly decomposes mail
  receipt through multiple daemon programs and queues:
  <https://www.postfix.org/TUNING_README.html> and
  <https://www.postfix.org/OVERVIEW.html>.
- Dovecot's own tuning docs say heavily loaded IMAP/POP3 servers are often
  disk-I/O bound, and its IMAP benchmarking guidance warns that realistic IMAP
  results depend heavily on client caching behavior, mailbox contents, metadata
  caching, and fsync policy:
  <https://doc.dovecot.org/2.3/configuration_manual/performance_tuning/> and
  <https://dovecot.github.io/imaptest/benchmarking.html>.
- OpenSMTPD publishes a small manual surface centered on `smtpd`,
  `smtpd.conf`, `smtpctl`, tables, aliases, forwarding, and filters:
  <https://www.opensmtpd.org/manual.html>.
- Exim exposes many queue and SMTP acceptance controls, including queue-only
  modes, queue-run limits, and SMTP accept queueing. The previous minimal Exim
  container is now historical evidence only, because the harness tripped over
  privilege/configuration issues before producing a fair concurrency result:
  <https://www.exim.org/exim-html-current/doc/html/spec_html/>.
- Maddy is a modern integrated Go mail server with SMTP and IMAP in one daemon,
  and now has a working local-only rootless harness target:
  <https://maddy.email/>.
- Mox is a modern integrated Go mail server. The harness can exercise its
  localserve development/test mode, but production comparison should wait until
  quickstart domain/account setup is automated:
  <https://www.xmox.nl/>.
- Stalwart is an important modern integrated comparator. Its docs position
  performance around integrated caches for email metadata, auth, DNS, and other
  data, so a fair benchmark needs reproducible first-run domain/account/storage
  setup rather than just container startup. The harness now automates a minimal
  first-run setup through `stalwart-cli`; the next step is production-shape
  policy tuning. The Docker docs for v0.16 describe bootstrap mode, recovery
  credentials, and the setup wizard; the CLI docs describe `stalwart-cli` and
  its declarative `apply` workflow:
  <https://stalw.art/docs/install/performance/>,
  <https://stalw.art/docs/install/platform/docker/> and
  <https://stalw.art/docs/management/cli/overview/>.

## Proposed Public Claim

Safe current wording:

> Edgerun is a compact integrated mail stack worth considering for operators who
> want SMTP, IMAP, web, DNS, DKIM, queue handling, and site hosting in one small
> service. In the latest rootless Podman local-delivery benchmark, Edgerun has
> higher SMTP throughput than the measured Maddy and Postfix comparators, faster
> basic IMAP than Maddy, and a much smaller image and process footprint. The
> next benchmark work is to prove whether that result holds for independent
> cold starts, connection reuse, realistic IMAP fetch/search, webmail, outbound
> delivery, Mox production setup, and Stalwart production policy.

Avoid these claims for now:

- "Edgerun is faster than Postfix" without limiting the claim to the measured
  local-delivery workload.
- "Edgerun beats Dovecot on IMAP."
- "Edgerun beats Maddy" without limiting the claim to the measured local SMTP
  and basic IMAP workloads.
- "Edgerun beats Mox."
- "Edgerun beats Stalwart."
- "Exim/OpenSMTPD are slow" based on the current minimal or historical harness
  configs.

## Research Plan

1. Add independent cold-start repetitions for Edgerun, Maddy, and Postfix with
   CPU samples, disk bytes, filesystem details, git commit, and dirty-tree
   status captured. The current five-run bundle is sustained/growing-state
   evidence, not independent cold-start evidence.
2. Add connection-reuse SMTP workload in addition to one-connection-per-message.
3. Add populated-mailbox IMAP workloads: login/list/select, fetch headers, fetch
   bodies, append, search, and idle.
4. Repeat the Maddy 10k SMTP run with warmup and repetitions, then add the same
   populated-mailbox IMAP workloads used for Edgerun/Postfix/Dovecot.
5. Automate Mox first-run domain/account/anti-relay setup and run the same SMTP
   and IMAP workloads. Keep localserve results labeled as development/test mode.
6. Tune Stalwart's local SMTP policy and concurrency behavior, then repeat the
   same SMTP and IMAP workloads at realistic scale. Keep the current v0.16
   setup-smoke bundle labeled as low-concurrency evidence.
7. Tune OpenSMTPD enough that failures/timeouts are either resolved or clearly
   documented as configuration findings. Exim is removed from the active
   comparison set until there is a reason to spend more time on its harness.
8. Add cold deployment evidence: image pull/build time, first listener readiness,
   config files, config lines, processes, exposed ports, and security-sensitive
   setup decisions.
9. Add production-shape evidence separately: Edgerun with full listeners enabled,
   TLS, DKIM, queue worker, DNS, webmail, blog, and Git host. Label this as a
   production snapshot, not a direct speed comparator.

## Optimization Hypotheses

Implementation work should focus on evidence from the rootless run:

- SMTP hot path: buffered session reads, multi-threaded runtime execution,
  recipient-batch delivery, batched status-index flushing, and default
  non-strict per-message fsync behavior improved the current 10k run enough to
  put Edgerun ahead of Maddy and Postfix in the repeated local-delivery bundle.
- SMTP session handling: measure whether connection setup dominates the current
  one-message-per-connection workload.
- IMAP path: buffered reads fix small-mailbox protocol overhead, and the
  mutation-maintained Maildir status sidecar fixes the current populated-mailbox
  SELECT smoke after 10k local deliveries. The next IMAP work should add a
  fuller mailbox index for FETCH/SEARCH/SORT, where message metadata and bodies
  still require filesystem scans and per-message parsing.
- Queue design: the previous queue-backed local delivery wrapper doubled disk
  writes, so any new queue optimization should batch or change durability
  semantics deliberately.
