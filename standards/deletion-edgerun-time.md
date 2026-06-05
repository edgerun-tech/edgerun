# edgerun-time deletion

Status: crossed out.

`crates/utility/edgerun-time` had no independent standard behavior worth
preserving as a Rust crate.

Deleted files:

- `crates/utility/edgerun-time/Cargo.toml`
- `crates/utility/edgerun-time/src/lib.rs`

Removed manifest hooks:

- root workspace member `crates/utility/edgerun-time`
- root workspace dependency `edgerun-time`

## Actual behavior found

- RFC3339 parse, format, and canonicalization were reexports of
  `edgerun_encoding::rfc3339`, whose useful behavior is owned by
  `time-rfc3339.wat`.
- `now_unix_millis` and `now_unix_micros` were host clock glue.
- The `chrono` feature was a compatibility layer over UTC seconds, duration
  arithmetic, partial date formatting, and RFC3339 formatting. It is not a
  portable standard primitive to preserve as a crate API.

## Replacement ownership

- RFC3339 scan/format/canonicalization: `time-rfc3339.wat`.
- Host clock reads: owner-local runtime/app clock helpers at the caller
  boundary.
- Duration freshness checks, test timestamp construction, and chrono-shaped
  wrappers: owner-local code in the Codex/catalog/protocol modules if still
  needed. Do not recreate a shared chrono compatibility crate.

## Remaining intentional refs

Live refs to `edgerun_time` / `edgerun-time` are caller-demolition targets. They
should be removed by replacing host clock calls locally or by projecting
RFC3339 through `time-rfc3339.wat`; they should not be fixed by restoring this
crate.
