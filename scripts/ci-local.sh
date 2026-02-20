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
    echo "  cargo test -p edgerun-runtime"
    echo "  cargo test -p edgerun-worker"
    echo "  ./scripts/integration_scheduler_api.sh"
    echo "  ./scripts/integration_e2e_lifecycle.sh"
    echo "  ./scripts/integration_policy_rotation.sh"
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
  cargo test -p edgerun-runtime
  cargo test -p edgerun-worker
}

run_integration() {
  ./scripts/integration_scheduler_api.sh
  ./scripts/integration_e2e_lifecycle.sh
  ./scripts/integration_policy_rotation.sh
}

case "${JOB:-all}" in
  all)
    run_rust_checks
    run_integration
    ;;
  rust-checks)
    run_rust_checks
    ;;
  integration)
    run_integration
    ;;
  *)
    echo "unsupported job for fallback mode: $JOB" >&2
    exit 1
    ;;
esac
