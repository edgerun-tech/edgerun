# Email Stack Comparison Evidence

Captured at: 2026-04-30T04:22:12Z

This bundle is raw evidence for a comparison post. It intentionally records
operator-experience data before the prose is written:

- generated benchmark config for each stack,
- stack/component metrics from the harness,
- package/server versions from each built image,
- image footprint before and after the run,
- build time through the rootless Podman harness,
- run/start time through the rootless Podman harness,
- one-at-a-time protocol benchmark output,
- memory/process samples while protocol load is running,
- logs and final stats.

Workload:

- Edgerun rootless Podman: 100 local-only SMTP loopback deliveries and
  10 IMAP login/list/select/logout sessions, concurrency 8.
- SMTP-only stacks: 100 local-only loopback deliveries, concurrency
  8, recipient bench@example.test, no external delivery.
- Dovecot IMAP: 10 login/list/select/logout sessions, concurrency
  8, benchmark user bench.
- Protocol commands have a 120 second wall-clock cap. A timeout is
  recorded as exit status 124 in the corresponding raw output.
- Setup commands have a 300 second wall-clock cap.
- Protocol timeouts above 300 seconds are refused unless
  EDGERUN_EMAIL_STORY_ALLOW_LONG_TIMEOUT=1 is set.
- Measured repetitions per workload: 1.
- SMTP warmup operations before measured repetitions: 0.
- IMAP warmup operations before measured repetitions: 0.
- Mox and Stalwart: bootstrap/startup evidence only until first-run
  domain/account and anti-relay setup is automated reproducibly.
- Native Edgerun snapshot: disabled by default. Set
  EDGERUN_EMAIL_STORY_INCLUDE_NATIVE_EDGERUN=1 to collect it as production-shape
  evidence, not as a direct performance comparator. Native SMTP uses
  `edgerun-smtp-bench` when available and the shell protocol benchmark as a
  fallback.
- Stack selection: `maddy mox`. Override with EDGERUN_EMAIL_STORY_STACKS to run
  one pair or one stack at a time.

All command stdout/stderr files are under `raw/`. Timing metadata is stored in
`raw/*.timing`.
