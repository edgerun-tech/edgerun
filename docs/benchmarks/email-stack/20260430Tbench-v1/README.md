# Email Stack Benchmark Evidence

Captured at: 2026-04-30T00:40:44Z

Protocol workload:

- SMTP: local delivery only, localhost high port 2525, 50 messages,
  concurrency 4, recipient bench@example.test.
- IMAP: localhost high port 1143, 50 login/list/select/logout sessions,
  concurrency 4, user bench.
- Stalwart: image/startup evidence only in this capture until first-run
  domain/account/anti-relay configuration is automated.

All raw command outputs are under `raw/`. stderr is stored next to stdout with
the same base name and an `.err` suffix.
