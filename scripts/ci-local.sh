#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW_FILE="$ROOT_DIR/.github/workflows/ci.yml"
EVENT="${EVENT:-pull_request}"
JOB="${JOB:-}"
DRY_RUN=0
ACT_RUNNER_LABEL="${ACT_RUNNER_LABEL:-ubuntu-24.04}"
ACT_IMAGE="${ACT_IMAGE:-ghcr.io/catthehacker/ubuntu:act-24.04}"

usage() {
  cat <<'EOF'
Run GitHub CI workflow locally.

Usage:
  scripts/ci-local.sh [--job <job>] [--event <event>] [--dry-run]

Examples:
  scripts/ci-local.sh
  scripts/ci-local.sh --job rust-checks
  scripts/ci-local.sh --job integration
  scripts/ci-local.sh --job runtime-determinism
  scripts/ci-local.sh --job runtime-calibration
  scripts/ci-local.sh --job runtime-slo
  scripts/ci-local.sh --job runtime-fuzz-sanity
  scripts/ci-local.sh --job runtime-security
  scripts/ci-local.sh --dry-run
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --job)
      JOB="${2:-}"
      shift 2
      ;;
    --event)
      EVENT="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ ! -f "$WORKFLOW_FILE" ]]; then
  echo "workflow file missing: $WORKFLOW_FILE" >&2
  exit 1
fi

if [[ "$DRY_RUN" == "1" ]]; then
  echo "workflow=$WORKFLOW_FILE"
  echo "event=$EVENT"
  echo "job=${JOB:-<all>}"
  if command -v act >/dev/null 2>&1; then
    echo "act command: act $EVENT -W $WORKFLOW_FILE ${JOB:+-j $JOB} -P $ACT_RUNNER_LABEL=$ACT_IMAGE"
  else
    echo "act not found; fallback commands:"
    echo "  cargo fmt --all --check"
    echo "  cargo check --workspace"
    echo "  cargo clippy --workspace --all-targets -- -D warnings"
    echo "  cargo test --workspace"
    echo "  ./scripts/integration_scheduler_api.sh"
    echo "  ./scripts/integration_e2e_lifecycle.sh"
    echo "  ./scripts/integration_policy_rotation.sh"
    echo "  cargo run -p edgerun-runtime -- replay-corpus --profile local --artifact /tmp/replay-corpus.local.json --runs 3"
    echo "  cargo run -p edgerun-runtime -- calibrate-fuel --profile local --artifact /tmp/fuel-calibration.local.json --runs 3 --max-per-unit-spread 0.4"
    echo "  cargo run -p edgerun-runtime -- slo-smoke --profile local --artifact /tmp/slo-smoke.local.json --runs 50 --max-p95-ms 100 --min-ops-per-sec 30"
    echo "  (optional, with cargo-fuzz+nightly) (cd crates/edgerun-runtime/fuzz && cargo +nightly fuzz run fuzz_bundle_decode -- -max_total_time=15 && cargo +nightly fuzz run fuzz_validate_wasm -- -max_total_time=15 && cargo +nightly fuzz run fuzz_hostcall_boundary -- -max_total_time=15)"
    echo "  (optional, with cargo-audit) cargo audit"
    echo "  (optional, with cargo-cyclonedx) cargo cyclonedx --manifest-path crates/edgerun-runtime/Cargo.toml --format json --override-filename runtime-sbom && mv crates/edgerun-runtime/runtime-sbom.json /tmp/edgerun-runtime-security/runtime-sbom.json"
    echo "  (optional, with bun+anchor+solana) ./program/scripts/test-bun-local"
  fi
  exit 0
fi

if command -v act >/dev/null 2>&1; then
  cd "$ROOT_DIR"
  cmd=(act "$EVENT" -W "$WORKFLOW_FILE" -P "$ACT_RUNNER_LABEL=$ACT_IMAGE")
  if [[ -n "$JOB" ]]; then
    cmd+=(-j "$JOB")
  fi
  "${cmd[@]}"
  exit 0
fi

echo "act not found; running fallback local CI sequence"
cd "$ROOT_DIR"

run_rust_checks() {
  cargo fmt --all --check
  cargo check --workspace
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
}

run_integration() {
  ./scripts/integration_scheduler_api.sh
  ./scripts/integration_e2e_lifecycle.sh
  ./scripts/integration_policy_rotation.sh
}

run_runtime_determinism() {
  local artifact="${TMPDIR:-/tmp}/replay-corpus.local.json"
  cargo run -p edgerun-runtime -- replay-corpus --profile local --artifact "$artifact" --runs 3
}

run_runtime_calibration() {
  local artifact="${TMPDIR:-/tmp}/fuel-calibration.local.json"
  cargo run -p edgerun-runtime -- calibrate-fuel \
    --profile local \
    --artifact "$artifact" \
    --runs 3 \
    --max-per-unit-spread 0.4
}

run_runtime_slo() {
  local artifact="${TMPDIR:-/tmp}/slo-smoke.local.json"
  cargo run -p edgerun-runtime -- slo-smoke \
    --profile local \
    --artifact "$artifact" \
    --runs 50 \
    --max-p95-ms 100 \
    --min-ops-per-sec 30
}

run_runtime_fuzz_sanity() {
  if ! command -v cargo-fuzz >/dev/null 2>&1 && ! cargo fuzz --help >/dev/null 2>&1; then
    echo "cargo-fuzz not found; skipping runtime-fuzz-sanity fallback"
    return 0
  fi
  if ! rustup toolchain list | grep -q '^nightly'; then
    echo "nightly toolchain not found; skipping runtime-fuzz-sanity fallback"
    return 0
  fi
  (
    cd "$ROOT_DIR/crates/edgerun-runtime/fuzz"
    cargo +nightly fuzz run fuzz_bundle_decode -- -max_total_time=15
    cargo +nightly fuzz run fuzz_validate_wasm -- -max_total_time=15
    cargo +nightly fuzz run fuzz_hostcall_boundary -- -max_total_time=15
  )
}

run_runtime_security() {
  if ! command -v cargo-audit >/dev/null 2>&1 && ! cargo audit --help >/dev/null 2>&1; then
    echo "cargo-audit not found; skipping runtime-security fallback"
    return 0
  fi
  cargo audit
  if ! command -v cargo-cyclonedx >/dev/null 2>&1; then
    echo "cargo-cyclonedx not found; skipping SBOM generation in fallback"
    return 0
  fi
  local out_dir="${TMPDIR:-/tmp}/edgerun-runtime-security"
  mkdir -p "$out_dir"
  cargo cyclonedx \
    --manifest-path crates/edgerun-runtime/Cargo.toml \
    --format json \
    --override-filename runtime-sbom
  mv crates/edgerun-runtime/runtime-sbom.json "$out_dir/runtime-sbom.json"
}

run_program_localnet() {
  if ! command -v bun >/dev/null 2>&1; then
    echo "bun not found; skipping program-localnet fallback"
    return 0
  fi
  if ! command -v anchor >/dev/null 2>&1; then
    echo "anchor not found; skipping program-localnet fallback"
    return 0
  fi
  if ! command -v solana-test-validator >/dev/null 2>&1; then
    echo "solana-test-validator not found; skipping program-localnet fallback"
    return 0
  fi
  (cd "$ROOT_DIR/program" && ./scripts/test-bun-local)
}

case "${JOB:-all}" in
  all)
    run_rust_checks
    run_integration
    run_runtime_determinism
    run_runtime_calibration
    run_runtime_slo
    run_runtime_fuzz_sanity
    run_runtime_security
    run_program_localnet
    ;;
  rust-checks)
    run_rust_checks
    ;;
  integration)
    run_integration
    ;;
  runtime-determinism)
    run_runtime_determinism
    ;;
  runtime-calibration)
    run_runtime_calibration
    ;;
  runtime-slo)
    run_runtime_slo
    ;;
  runtime-fuzz-sanity)
    run_runtime_fuzz_sanity
    ;;
  runtime-security)
    run_runtime_security
    ;;
  program-localnet)
    run_program_localnet
    ;;
  *)
    echo "unsupported job for fallback mode: $JOB" >&2
    exit 1
    ;;
esac
