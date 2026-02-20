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
    run_program_localnet
    ;;
  rust-checks)
    run_rust_checks
    ;;
  integration)
    run_integration
    ;;
  program-localnet)
    run_program_localnet
    ;;
  *)
    echo "unsupported job for fallback mode: $JOB" >&2
    exit 1
    ;;
esac
