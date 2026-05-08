#!/usr/bin/env sh
set -eu

root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$root"

out_dir="${COVERAGE_DIR:-target/coverage}"
rust_dir="$out_dir/rust"
run_rust=1
run_frontend=1
require_rust=0
rust_package_args=""
rust_scope_args="--workspace"

usage() {
  cat <<'USAGE'
usage: scripts/coverage-report.sh [--rust-only|--frontend-only] [--require-rust-tool]

Generate local coverage reports.

  --rust-only          generate only Rust coverage
  --frontend-only      generate only frontend coverage
  --require-rust-tool  fail if cargo-llvm-cov is not installed

Environment:
  COVERAGE_DIR             output directory; default: target/coverage
  RUST_COVERAGE_PACKAGES   optional space-separated package list for Rust coverage
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --rust-only)
      run_frontend=0
      ;;
    --frontend-only)
      run_rust=0
      ;;
    --require-rust-tool)
      require_rust=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

mkdir -p "$rust_dir"

if [ -n "${RUST_COVERAGE_PACKAGES:-}" ]; then
  for package in $RUST_COVERAGE_PACKAGES; do
    rust_package_args="$rust_package_args -p $package"
  done
  rust_scope_args="$rust_package_args"
fi

if [ "$run_rust" -eq 1 ] && command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "Generating Rust coverage report..."
  cargo llvm-cov clean --workspace
  # shellcheck disable=SC2086
  cargo llvm-cov \
    $rust_scope_args \
    --html \
    --output-dir "$rust_dir/html"
  # shellcheck disable=SC2086
  cargo llvm-cov report \
    $rust_scope_args \
    --lcov \
    --output-path "$rust_dir/lcov.info"
  echo "Rust coverage HTML: $rust_dir/html/index.html"
  echo "Rust coverage LCOV: $rust_dir/lcov.info"
elif [ "$run_rust" -eq 1 ]; then
  echo "cargo-llvm-cov is not installed; skipping Rust coverage." >&2
  echo "Install it with: cargo install cargo-llvm-cov" >&2
  if [ "$require_rust" -eq 1 ]; then
    exit 1
  fi
fi

if [ "$run_frontend" -eq 1 ] && [ -d frontend ] && [ -f frontend/package.json ]; then
  echo "Generating frontend coverage report..."
  (cd frontend && bun run test:coverage)
  echo "Frontend coverage HTML: frontend/coverage/index.html"
fi
