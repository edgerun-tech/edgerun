#!/bin/bash
# edgerun-oci conformance test runner
set -euo pipefail
export PATH="/usr/local/sbin:/usr/local/bin:/usr/bin:/usr/lib/emscripten:/opt/android-sdk/platform-tools"
export HOME=/home/ken
export CARGO_HOME=/home/ken/.cargo
cd /home/ken/edgerun_reference_core

# Clean up any previous state
pkill -9 -f edgerun-oci 2>/dev/null || true
rm -rf /run/edgerun-oci /tmp/oci-* 2>/dev/null || true

LOG="/var/log/edgerun-oci-conformance.log"

echo "=== Starting conformance tests $(date) ===" | tee "$LOG"
echo "" | tee -a "$LOG"

# Build first
/usr/bin/cargo build -p edgerun-oci-runtime --release 2>&1 | tee -a "$LOG"

# Run conformance tests SEQUENTIALLY (--test-threads=1)
# Parallel execution causes mount namespace and cgroup conflicts
/usr/bin/cargo test -p edgerun-oci-runtime --test conformance -- --ignored --nocapture --test-threads=1 2>&1 | tee -a "$LOG"
RC=$?

echo "" | tee -a "$LOG"
echo "=== Completed conformance tests $(date), exit=$RC ===" | tee -a "$LOG"

exit $RC
