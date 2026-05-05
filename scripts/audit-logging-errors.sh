#!/usr/bin/env sh
set -eu

root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$root"

fail=0

echo "Checking runtime logging paths..."
logging_matches="$(
  rg -n '(^|[^[:alnum:]_])(println!|eprintln!|dbg!)|(^|[^[:alnum:]_])log::(trace|debug|info|warn|error)!|tracing::' crates \
  --glob '*.rs' \
  --glob '!**/tests/**' \
  --glob '!**/benches/**' \
  --glob '!**/*bench*.rs' \
  --glob '!**/*benchmark*.rs' \
  --glob '!**/*conformance*.rs' \
  --glob '!**/bin/**' \
  --glob '!**/src/main.rs' \
  --glob '!**/src/cli/**' \
  --glob '!**/src/cli.rs' \
  --glob '!**/src/init_cmd.rs' \
  --glob '!**/src/status_cmd.rs' \
  --glob '!**/src/signer.rs' \
  --glob '!**/src/config.rs' \
  --glob '!**/build.rs' \
  --glob '!**/vendor/**' \
  --glob '!crates/edgerun-bench/**' \
  --glob '!crates/utility/edgerun-clap/**' \
  --glob '!crates/edgerun-e2e/**' \
  --glob '!crates/edgerun-marketplace-cli/**' \
  --glob '!crates/edgerun-oci/src/cli/**' \
  --glob '!crates/edgerun-oci/tests/contest/**' \
  --glob '!crates/edgerun-core/src/lib.rs' || true
)"
logging_matches="$(printf '%s\n' "$logging_matches" | grep -v ':[[:space:]]*///' | grep -v ':[[:space:]]*//!' | grep -v ':[[:space:]]*//' || true)"
if [ -n "$logging_matches" ]; then
  printf '%s\n' "$logging_matches"
  fail=1
fi

echo "Checking external logging/error crates..."
dependency_matches="$(
  rg -n '(^|[[:space:]])(thiserror|tracing|tracing-subscriber|env_logger|simple_logger|slog|log)[[:space:]]*=' Cargo.toml crates/*/Cargo.toml \
    --glob '!crates/edgerun-oci/tests/contest/**' || true
)"
if [ -n "$dependency_matches" ]; then
  printf '%s\n' "$dependency_matches"
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "Found non-central logging or external error/logging dependencies." >&2
  exit 1
fi

echo "Logging and error dependency audit passed."
