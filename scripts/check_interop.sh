#!/usr/bin/env bash
set -euo pipefail
export GOCACHE="${GOCACHE:-$(pwd)/var/go-build-cache}"
cd "$(dirname "$0")/.."

(
  cd go
  GOFLAGS="${GOFLAGS:-}" go run ./cmd/run-conformance --corpus ../corpus/vectors-v0.1
)

echo "conformance check complete"
