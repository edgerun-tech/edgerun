# Email Stack Comparison Evidence

Captured at: 2026-04-30T03:05:35Z

This bundle is raw evidence for a comparison post. It intentionally records
operator-experience data before the prose is written:

- generated benchmark config for each stack,
- stack/component metrics from the harness,
- image footprint before and after the run,
- build time through the rootless Podman harness,
- run/start time through the rootless Podman harness,
- one-at-a-time protocol benchmark output,
- memory/process samples while protocol load is running,
- logs and final stats.

Workload:

- Edgerun rootless Podman: 10000 local-only SMTP loopback deliveries and
  100 IMAP login/list/select/logout sessions, concurrency 32.
- SMTP-only stacks: 10000 local-only loopback deliveries, concurrency
  32, recipient bench@example.test, no external delivery.
- Dovecot IMAP: 100 login/list/select/logout sessions, concurrency
  32, benchmark user bench.
- Protocol commands have a 600 second wall-clock cap. A timeout is
  recorded as exit status 124 in the corresponding raw output.
- Stalwart: bootstrap/startup evidence only until first-run domain/account and
  anti-relay setup is automated reproducibly.
- Native Edgerun snapshot: disabled by default. Set
  EDGERUN_EMAIL_STORY_INCLUDE_NATIVE_EDGERUN=1 to collect it as production-shape
  evidence, not as a direct performance comparator.
- Stack selection: `edgerun postfix`. Override with EDGERUN_EMAIL_STORY_STACKS to run
  one pair or one stack at a time.

All command stdout/stderr files are under `raw/`. Timing metadata is stored in
`raw/*.timing`.
