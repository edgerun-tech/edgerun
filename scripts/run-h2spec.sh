#!/usr/bin/env bash
# run-h2spec.sh — Automated h2spec conformance testing for edgerun-http
#
# Usage:
#   ./scripts/run-h2spec.sh              # Run against localhost default port
#   ./scripts/run-h2spec.sh --port 9999  # Custom port
#   ./scripts/run-h2spec.sh --strict     # Run strict tests too
#
# Prerequisites:
#   - h2spec binary on PATH (go install github.com/summerwind/h2spec/cmd/h2spec@latest)
#   - cargo (to build the h2spec-server binary)
#   - edgerun-http crate with "tls" feature
#
# Exit codes:
#   0 — All tests passed
#   1 — Some tests failed
#   2 — h2spec not found
#   3 — Server failed to start

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Defaults
PORT=0
STRICT=""
H2SPEC_ARGS=()
REPORT_PATH=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --port)
            PORT="$2"; shift 2 ;;
        --strict)
            STRICT="--strict"; shift ;;
        --help|-h)
            echo "Usage: $0 [--port PORT] [--strict]"
            exit 0 ;;
        *)
            echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Find h2spec
H2SPEC_BIN="$(command -v h2spec 2>/dev/null || true)"
if [[ -z "$H2SPEC_BIN" ]]; then
    echo "ERROR: h2spec not found on PATH"
    echo "Install with: go install github.com/summerwind/h2spec/cmd/h2spec@latest"
    exit 2
fi

echo "=== h2spec Conformance Test ==="
echo "h2spec: $H2SPEC_BIN ($($H2SPEC_BIN --version 2>&1))"

# Build the server
echo ""
echo "Building h2spec-server..."
cd "$PROJECT_ROOT"
cargo build -p edgerun-http --features tls --bin h2spec-server 2>&1 | grep -E "^(error|Compiling|Finished)" || true

SERVER_BIN="$PROJECT_ROOT/target/debug/h2spec-server"
if [[ ! -x "$SERVER_BIN" ]]; then
    echo "ERROR: h2spec-server binary not found at $SERVER_BIN"
    exit 3
fi

# Pick a port
if [[ "$PORT" -eq 0 ]]; then
    # Find a free port
    PORT=$(python3 -c "import socket; s=socket.socket(); s.bind(('',0)); print(s.getsockname()[1]); s.close()" 2>/dev/null || echo 19876)
fi

echo "Port: $PORT"
if [[ -n "$STRICT" ]]; then
    echo "Mode: strict (includes strict test cases)"
fi

# Start server
echo ""
echo "Starting h2spec-server on 127.0.0.1:$PORT ..."
"$SERVER_BIN" --port "$PORT" &
SERVER_PID=$!

# Wait for server to be ready
sleep 1
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "ERROR: Server failed to start"
    exit 3
fi

echo "Server PID: $SERVER_PID"

# Timestamp for report
TIMESTAMP="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
REPORT_FILE="$PROJECT_ROOT/target/h2spec-report-${TIMESTAMP//[:T]/-}.xml"

# Run h2spec
echo ""
echo "Running h2spec..."
echo "Command: h2spec -h 127.0.0.1 -p $PORT -t $STRICT -j $REPORT_FILE"
echo "────────────────────────────────────────────"

H2SPEC_EXIT=0
set +e
$H2SPEC_BIN -h 127.0.0.1 -p "$PORT" -k -t -o 5 $STRICT -j "$REPORT_FILE" 2>&1
H2SPEC_EXIT=$?
set -e

echo "────────────────────────────────────────────"

# Kill server
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true

# Report results
echo ""
echo "=== Results ==="
if [[ -f "$REPORT_FILE" ]]; then
    # Parse JUnit XML
    TOTAL=$(grep -oP 'tests="\K[0-9]+' "$REPORT_FILE" | head -1 || echo "0")
    FAILURES=$(grep -oP 'failures="\K[0-9]+' "$REPORT_FILE" | head -1 || echo "0")
    ERRORS=$(grep -oP 'errors="\K[0-9]+' "$REPORT_FILE" | head -1 || echo "0")
    SKIPPED=$(grep -oP 'skipped="\K[0-9]+' "$REPORT_FILE" | head -1 || echo "0")
    PASSED=$((TOTAL - FAILURES - ERRORS - SKIPPED))

    echo "Total:    $TOTAL"
    echo "Passed:   $PASSED"
    echo "Failed:   $FAILURES"
    echo "Errors:   $ERRORS"
    echo "Skipped:  $SKIPPED"
    echo ""
    echo "Report: $REPORT_FILE"

    # Show failures
    if [[ "$FAILURES" -gt 0 || "$ERRORS" -gt 0 ]]; then
        echo ""
        echo "=== Failures ==="
        grep -A5 '<failure\|<error' "$REPORT_FILE" | head -50 || true
    fi
else
    echo "No JUnit report generated"
    echo "h2spec exit code: $H2SPEC_EXIT"
fi

# Exit with test result
if [[ "$H2SPEC_EXIT" -eq 0 ]]; then
    echo ""
    echo "✅ All h2spec tests passed!"
    exit 0
else
    echo ""
    echo "❌ h2spec tests failed (exit code: $H2SPEC_EXIT)"
    exit 1
fi
