#!/usr/bin/env bash
# e2e-test-runner.sh — Comprehensive end-to-end test runner for all edgerun subsystems
#
# This script runs the full E2E test suite, reporting results for each subsystem:
#   1. Node Daemon Lifecycle (init, start, stop, health)
#   2. TCP Session Establishment (handshake, rejection, validation)
#   3. Command Dispatch & Validation (query, rejection, signature)
#   4. Storage & Event Stream (genesis, persistence)
#   5. Local Capability Server (Unix socket capability server)
#   6. Ingress Screening & Rate Limiting (connection limits)
#   7. Mesh Networking (frame encoding, discovery, routing - software only)
#   8. Protocol Conformance (protobuf roundtrip, crypto operations)
#   9. Full Integration (multi-node sessions, commands)
#  10. Hardware Capability Tests (requires HARDWARE_E2E=1)
#  11. OCI Workload Execution (native Linux namespaces, no external deps)
#  12. Mesh Integration (network namespaces, veth pairs - requires root)
#
# Usage:
#   ./scripts/e2e-test-runner.sh              # Run software tests only
#   HARDWARE_E2E=1 ./scripts/e2e-test-runner.sh  # Include hardware tests
#   ./scripts/e2e-test-runner.sh --mesh       # Include mesh namespace tests
#   ./scripts/e2e-test-runner.sh --all        # Run everything
#   ./scripts/e2e-test-runner.sh --help       # Show help

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

PASS=0
FAIL=0
SKIP=0
TOTAL=0
SUBSYSTEM_PASS=()
SUBSYSTEM_FAIL=()
SUBSYSTEM_SKIP=()
SUBSYSTEM_NAMES=()

RUN_HARDWARE=${HARDWARE_E2E:-0}
RUN_MESH=false
RUN_ALL=false
RUN_SOFTWARE=true

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --hardware)
            RUN_HARDWARE=1
            shift
            ;;
        --mesh)
            RUN_MESH=true
            shift
            ;;
        --all)
            RUN_ALL=true
            RUN_HARDWARE=1
            RUN_MESH=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--hardware] [--mesh] [--all] [--help]"
            echo ""
            echo "Options:"
            echo "  --hardware    Include hardware capability E2E tests"
            echo "  --mesh        Include mesh network namespace tests (requires root)"
            echo "  --all         Run all tests including hardware and mesh"
            echo "  --help        Show this help message"
            echo ""
            echo "Environment:"
            echo "  HARDWARE_E2E=1    Same as --hardware"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

pass() {
    PASS=$((PASS + 1))
    TOTAL=$((TOTAL + 1))
    echo -e "    ${GREEN}✓${NC} $1"
}

fail() {
    FAIL=$((FAIL + 1))
    TOTAL=$((TOTAL + 1))
    echo -e "    ${RED}✗${NC} $1"
    if [ -n "${2:-}" ]; then
        echo -e "      expected: $2"
    fi
}

skip() {
    SKIP=$((SKIP + 1))
    TOTAL=$((TOTAL + 1))
    echo -e "    ${YELLOW}⊘${NC} $1 (skipped)"
}

section() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
}

subsystem_start() {
    SUBSYSTEM_NAMES+=("$1")
    SUBSYSTEM_PASS=("${SUBSYSTEM_PASS[@]}" 0)
    SUBSYSTEM_FAIL=("${SUBSYSTEM_FAIL[@]}" 0)
    SUBSYSTEM_SKIP=("${SUBSYSTEM_SKIP[@]}" 0)
    echo ""
    echo -e "${CYAN}─── $1 ───${NC}"
}

subsystem_record() {
    local status=$1
    local idx=$(( ${#SUBSYSTEM_NAMES[@]} - 1 ))
    case "$status" in
        pass)
            SUBSYSTEM_PASS[$idx]=$(( ${SUBSYSTEM_PASS[$idx]} + 1 ))
            ;;
        fail)
            SUBSYSTEM_FAIL[$idx]=$(( ${SUBSYSTEM_FAIL[$idx]} + 1 ))
            ;;
        skip)
            SUBSYSTEM_SKIP[$idx]=$(( ${SUBSYSTEM_SKIP[$idx]} + 1 ))
            ;;
    esac
}

subsystem_summary() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}SUBSYSTEM SUMMARY${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    
    for i in "${!SUBSYSTEM_NAMES[@]}"; do
        local name="${SUBSYSTEM_NAMES[$i]}"
        local p="${SUBSYSTEM_PASS[$i]}"
        local f="${SUBSYSTEM_FAIL[$i]}"
        local s="${SUBSYSTEM_SKIP[$i]}"
        
        if [ "$f" -gt 0 ]; then
            echo -e "  ${RED}✗${NC} $name: ${p} passed, ${RED}${f} failed${NC}, ${s} skipped"
        elif [ "$p" -gt 0 ]; then
            echo -e "  ${GREEN}✓${NC} $name: ${GREEN}${p} passed${NC}, ${f} failed, ${s} skipped"
        else
            echo -e "  ${YELLOW}⊘${NC} $name: ${p} passed, ${f} failed, ${YELLOW}${s} skipped${NC}"
        fi
    done
}

cleanup() {
    # Kill any remaining background processes
    jobs -p 2>/dev/null | xargs -r kill 2>/dev/null || true
}

trap cleanup EXIT

# ────────────────────────────────────────────────────────────────
# Pre-flight checks
# ────────────────────────────────────────────────────────────────
section "Pre-flight Checks"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

echo -n "  Checking cargo... "
if command -v cargo &>/dev/null; then
    echo -e "${GREEN}ok${NC}"
    pass "cargo available"
else
    echo -e "${RED}not found${NC}"
    fail "cargo not available"
    exit 1
fi

echo -n "  Checking Rust toolchain... "
if rustc --version &>/dev/null; then
    echo -e "${GREEN}$(rustc --version)${NC}"
    pass "Rust toolchain available"
else
    echo -e "${RED}not found${NC}"
    fail "Rust toolchain not available"
    exit 1
fi

echo -n "  Checking edgerund binary... "
if [ -f "target/debug/edgerund" ]; then
    echo -e "${GREEN}ok${NC}"
    pass "edgerund binary exists"
else
    echo -e "${YELLOW}not found, will build on demand${NC}"
    skip "edgerund binary not pre-built"
fi

echo -n "  Checking workspace integrity... "
if cargo check --workspace --quiet 2>/dev/null; then
    echo -e "${GREEN}ok${NC}"
    pass "workspace integrity verified"
else
    echo -e "${YELLOW}check issues (continuing)${NC}"
    fail "workspace check failed"
fi

# ────────────────────────────────────────────────────────────────
# Build
# ────────────────────────────────────────────────────────────────
section "Building Test Binaries"

echo -n "  Building edgerund... "
if cargo build -p edgerun-node --quiet 2>&1; then
    echo -e "${GREEN}ok${NC}"
    pass "edgerund built successfully"
else
    echo -e "${RED}failed${NC}"
    fail "edgerund build failed"
    exit 1
fi

echo -n "  Building E2E test crate... "
if cargo test -p edgerun-e2e --no-run --quiet 2>&1; then
    echo -e "${GREEN}ok${NC}"
    pass "E2E test crate built"
else
    echo -e "${RED}failed${NC}"
    fail "E2E test build failed"
    exit 1
fi

# ────────────────────────────────────────────────────────────────
# Run E2E Tests
# ────────────────────────────────────────────────────────────────
section "Running End-to-End Tests"

echo -e "  ${YELLOW}Note: All tests are #[ignore] and require --ignored flag${NC}"
echo -e "  Running with: ${CYAN}cargo test -p edgerun-e2e -- --ignored${NC}"
echo ""

# Run the E2E tests and capture output
set +e
TEST_OUTPUT=$(cargo test -p edgerun-e2e -- --ignored --nocapture 2>&1)
TEST_EXIT=$?
set -e

# Parse the output
while IFS= read -r line; do
    if echo "$line" | grep -q "test .* \.\.\. ok"; then
        test_name=$(echo "$line" | grep -oP 'test \K[^ ]+')
        pass "$test_name"
        subsystem_record "pass"
    elif echo "$line" | grep -q "test .* \.\.\. FAILED"; then
        test_name=$(echo "$line" | grep -oP 'test \K[^ ]+')
        fail "$test_name"
        subsystem_record "fail"
    fi
done <<< "$TEST_OUTPUT"

# Print the full output for debugging
if [ $TEST_EXIT -eq 0 ]; then
    echo -e "\n  ${GREEN}All E2E tests passed!${NC}"
else
    echo -e "\n  ${RED}Some E2E tests failed (exit code: $TEST_EXIT)${NC}"
    echo -e "  See details above for failure reasons\n"
fi

# ────────────────────────────────────────────────────────────────
# Hardware Tests (optional)
# ────────────────────────────────────────────────────────────────
if [ "$RUN_HARDWARE" = "1" ]; then
    section "Hardware Capability Tests (HARDWARE_E2E=1)"
    
    subsystem_start "Hardware Capabilities"
    
    echo -e "  ${YELLOW}Running hardware E2E tests...${NC}"
    set +e
    HW_OUTPUT=$(HARDWARE_E2E=1 cargo test -p edgerun-e2e-capability -- --ignored --nocapture 2>&1)
    HW_EXIT=$?
    set -e
    
    while IFS= read -r line; do
        if echo "$line" | grep -q "test .* \.\.\. ok"; then
            test_name=$(echo "$line" | grep -oP 'test \K[^ ]+')
            pass "$test_name"
            subsystem_record "pass"
        elif echo "$line" | grep -q "test .* \.\.\. FAILED"; then
            test_name=$(echo "$line" | grep -oP 'test \K[^ ]+')
            fail "$test_name"
            subsystem_record "fail"
        elif echo "$line" | grep -q "test .* \.\.\. ignored"; then
            test_name=$(echo "$line" | grep -oP 'test \K[^ ]+')
            skip "$test_name"
            subsystem_record "skip"
        fi
    done <<< "$HW_OUTPUT"
    
    if [ $HW_EXIT -eq 0 ]; then
        echo -e "\n  ${GREEN}Hardware tests completed!${NC}"
    else
        echo -e "\n  ${YELLOW}Some hardware tests failed or were skipped (exit: $HW_EXIT)${NC}"
    fi
else
    echo ""
    echo -e "${YELLOW}Skipping hardware tests (set HARDWARE_E2E=1 or use --hardware)${NC}"
fi

# ────────────────────────────────────────────────────────────────
# Mesh Integration Tests (optional, requires root)
# ────────────────────────────────────────────────────────────────
if [ "$RUN_MESH" = "true" ]; then
    section "Mesh Integration Tests (Network Namespaces)"
    
    if [ "$(id -u)" -ne 0 ]; then
        echo -e "  ${YELLOW}Mesh tests require root. Skipping (or run with sudo)${NC}"
        skip "Mesh integration tests (requires root)"
    else
        echo -e "  ${YELLOW}Running mesh integration test...${NC}"
        set +e
        MESH_OUTPUT=$(bash "$SCRIPT_DIR/mesh-integration-test.sh" 2>&1)
        MESH_EXIT=$?
        set -e
        
        while IFS= read -r line; do
            if echo "$line" | grep -q "✓"; then
                test_name=$(echo "$line" | sed 's/.*✓ //')
                pass "$test_name"
                subsystem_record "pass"
            elif echo "$line" | grep -q "✗"; then
                test_name=$(echo "$line" | sed 's/.*✗ //')
                fail "$test_name"
                subsystem_record "fail"
            fi
        done <<< "$MESH_OUTPUT"
        
        if [ $MESH_EXIT -eq 0 ]; then
            echo -e "\n  ${GREEN}Mesh integration tests passed!${NC}"
        else
            echo -e "\n  ${RED}Mesh integration tests failed (exit: $MESH_EXIT)${NC}"
        fi
    fi
else
    echo -e "\n${YELLOW}Skipping mesh namespace tests (use --mesh or --all)${NC}"
fi

# ────────────────────────────────────────────────────────────────
# Conformance Corpus Tests
# ────────────────────────────────────────────────────────────────
section "Protocol Conformance Corpus"

echo -e "  Running conformance tests..."
set +e
CONFORMANCE_OUTPUT=$(cargo test -- conformance --ignored --nocapture 2>&1)
CONFORMANCE_EXIT=$?
set -e

while IFS= read -r line; do
    if echo "$line" | grep -q "test .* \.\.\. ok"; then
        test_name=$(echo "$line" | grep -oP 'test \K[^ ]+')
        pass "$conformance test"
        subsystem_record "pass"
    elif echo "$line" | grep -q "test .* \.\.\. FAILED"; then
        test_name=$(echo "$line" | grep -oP 'test \K[^ ]+')
        fail "$conformance test"
        subsystem_record "fail"
    fi
done <<< "$CONFORMANCE_OUTPUT"

# ────────────────────────────────────────────────────────────────
# Final Summary
# ────────────────────────────────────────────────────────────────
section "Final Test Summary"

echo -e "  ${CYAN}Total Tests:${NC} $TOTAL"
echo -e "  ${GREEN}Passed:${NC}      $PASS"
echo -e "  ${RED}Failed:${NC}      $FAIL"
echo -e "  ${YELLOW}Skipped:${NC}     $SKIP"

if [ $FAIL -eq 0 ]; then
    echo -e "\n  ${GREEN}✓ ALL TESTS PASSED${NC}"
    exit 0
else
    echo -e "\n  ${RED}✗ $FAIL TEST(S) FAILED${NC}"
    subsystem_summary
    exit 1
fi
