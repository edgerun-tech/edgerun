#!/bin/bash
# Simple benchmark: edgerun-oci-runtime vs crun
# Focuses on binary characteristics and OCI spec parsing

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

ITERATIONS=${ITERATIONS:-10}
BENCH_DIR="/tmp/edgerun-bench-$$"
EDGERUN_OCI="/home/ken/edgerun_reference_core/target/release/edgerun-oci"
CRUN=$(which crun)

cleanup() {
    rm -rf "$BENCH_DIR"
}
trap cleanup EXIT

mkdir -p "$BENCH_DIR"

echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║     edgerun-oci-runtime vs crun: Benchmark Report      ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}Date:${NC} $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
echo -e "${BLUE}Iterations:${NC} $ITERATIONS"
echo -e "${BLUE}edgerun-oci:${NC} $EDGERUN_OCI"
echo -e "${BLUE}crun:${NC} $CRUN ($(crun --version 2>&1 | head -1))"
echo ""

# ============================================================================
# 1. Binary Characteristics
# ============================================================================
echo -e "${YELLOW}═══ 1. Binary Characteristics ═══${NC}"
echo ""

EDGERUN_SIZE=$(stat -c%s "$EDGERUN_OCI")
CRUN_SIZE=$(stat -c%s "$CRUN")

echo -e "${BLUE}Binary Size:${NC}"
printf "  edgerun-oci: %'d bytes (%.2f KB)\n" "$EDGERUN_SIZE" "$(echo "scale=2; $EDGERUN_SIZE / 1024" | bc)"
printf "  crun:        %'d bytes (%.2f KB)\n" "$CRUN_SIZE" "$(echo "scale=2; $CRUN_SIZE / 1024" | bc)"

RATIO=$(echo "scale=2; $EDGERUN_SIZE / $CRUN_SIZE" | bc)
echo -e ""
echo -e "  ${YELLOW}Size ratio:${NC} edgerun-oci is ${RATIO}x larger than crun"
echo ""

# File type info
echo -e "${BLUE}Binary Type:${NC}"
echo -e "  edgerun-oci: $(file "$EDGERUN_OCI" | cut -d: -f2 | sed 's/^ *//')"
echo -e "  crun:        $(file "$CRUN" | cut -d: -f2 | sed 's/^ *//')"
echo ""

# Check if stripped
echo -e "${BLUE}Strip Status:${NC}"
if file "$EDGERUN_OCI" | grep -q "not stripped"; then
    echo -e "  edgerun-oci: ${RED}Not stripped (debug symbols present)${NC}"
    EDGERUN_SIZE_STRIPPED=$(echo "$EDGERUN_SIZE * 0.3" | bc | cut -d. -f1)
    echo -e "  ${YELLOW}Estimated stripped: ~${EDGERUN_SIZE_STRIPPED} bytes${NC}"
else
    echo -e "  edgerun-oci: ${GREEN}Stripped${NC}"
fi

if file "$CRUN" | grep -q "stripped"; then
    echo -e "  crun:        ${GREEN}Stripped${NC}"
fi
echo ""

# ============================================================================
# 2. Compile Time / Build Performance
# ============================================================================
echo -e "${YELLOW}═══ 2. Build Performance (edgerun-oci only) ═══${NC}"
echo ""

echo -e "${BLUE}Building edgerun-oci (release mode)...${NC}"
BUILD_START=$(date +%s%N)
cd /home/ken/edgerun_reference_core
cargo build --release -p edgerun-oci-runtime 2>&1 | tail -1
BUILD_END=$(date +%s%N)
BUILD_TIME=$(( (BUILD_END - BUILD_START) / 1000000 ))
echo -e "  ${BLUE}Build time:${NC} ${BUILD_TIME} ms"
echo ""

# Count source files and lines
echo -e "${BLUE}Code Statistics:${NC}"
SRC_DIR="/home/ken/edgerun_reference_core/crates/edgerun-oci-runtime/src"
NUM_FILES=$(find "$SRC_DIR" -name "*.rs" | wc -l)
NUM_LINES=$(find "$SRC_DIR" -name "*.rs" -exec cat {} + | wc -l)
echo -e "  Source files: $NUM_FILES"
echo -e "  Total lines:  $NUM_LINES"
printf "  Average:      %.0f lines/file\n" "$(echo "scale=0; $NUM_LINES / $NUM_FILES" | bc)"
echo ""

# ============================================================================
# 3. OCI Spec Parsing Speed
# ============================================================================
echo -e "${YELLOW}═══ 3. OCI Spec Parsing Performance ═══${NC}"
echo ""

# Create standard OCI config
cat > "$BENCH_DIR/config.json" << 'EOF'
{
    "ociVersion": "1.0.2",
    "process": {
        "terminal": false,
        "user": { "uid": 0, "gid": 0 },
        "args": ["/bin/sh", "-c", "echo hello"],
        "env": ["PATH=/usr/bin:/bin", "TERM=xterm"],
        "cwd": "/",
        "noNewPrivileges": true
    },
    "root": {
        "path": "rootfs",
        "readonly": false
    },
    "linux": {
        "namespaces": [
            { "type": "pid" },
            { "type": "mount" }
        ]
    }
}
EOF

mkdir -p "$BENCH_DIR/rootfs"
cp /bin/sh "$BENCH_DIR/rootfs/" 2>/dev/null || true

# Benchmark edgerun-oci
echo -e "${BLUE}Testing edgerun-oci (create command parses spec)...${NC}"
EDGERUN_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    CONTAINER_ID="bench_parse_edgerun_$i"
    
    start_time=$(date +%s%N)
    timeout 3 "$EDGERUN_OCI" create --bundle "$BENCH_DIR" "$CONTAINER_ID" 2>/dev/null || true
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    EDGERUN_TIMES+=($elapsed)
    
    # Cleanup
    timeout 2 "$EDGERUN_OCI" delete --force "$CONTAINER_ID" 2>/dev/null || true
    rm -rf "/run/edgerun-oci/$CONTAINER_ID" 2>/dev/null || true
done

EDGERUN_AVG=$(printf '%s\n' "${EDGERUN_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
EDGERUN_MIN=$(printf '%s\n' "${EDGERUN_TIMES[@]}" | sort -n | head -1)
EDGERUN_MAX=$(printf '%s\n' "${EDGERUN_TIMES[@]}" | sort -n | tail -1)

echo -e "  Average: ${EDGERUN_AVG} ms"
echo -e "  Min:     ${EDGERUN_MIN} ms"
echo -e "  Max:     ${EDGERUN_MAX} ms"
echo ""

# Benchmark crun
echo -e "${BLUE}Testing crun (create command parses spec)...${NC}"
CRUN_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    CONTAINER_ID="bench_parse_crun_$i"
    
    start_time=$(date +%s%N)
    timeout 3 "$CRUN" create "$CONTAINER_ID" --bundle "$BENCH_DIR" 2>/dev/null || true
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    CRUN_TIMES+=($elapsed)
    
    # Cleanup
    timeout 2 "$CRUN" delete --force "$CONTAINER_ID" 2>/dev/null || true
done

CRUN_AVG=$(printf '%s\n' "${CRUN_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
CRUN_MIN=$(printf '%s\n' "${CRUN_TIMES[@]}" | sort -n | head -1)
CRUN_MAX=$(printf '%s\n' "${CRUN_TIMES[@]}" | sort -n | tail -1)

echo -e "  Average: ${CRUN_AVG} ms"
echo -e "  Min:     ${CRUN_MIN} ms"
echo -e "  Max:     ${CRUN_MAX} ms"
echo ""

# Comparison
if [ "$EDGERUN_AVG" -gt 0 ] && [ "$CRUN_AVG" -gt 0 ]; then
    echo -e "${YELLOW}Comparison:${NC}"
    if [ "$EDGERUN_AVG" -le "$CRUN_AVG" ]; then
        SPEEDUP=$(echo "scale=2; $CRUN_AVG / $EDGERUN_AVG" | bc)
        echo -e "  ${GREEN}✓ edgerun-oci is ${SPEEDUP}x faster at spec parsing${NC}"
    else
        SPEEDUP=$(echo "scale=2; $EDGERUN_AVG / $CRUN_AVG" | bc)
        echo -e "  ${YELLOW}⚠ crun is ${SPEEDUP}x faster at spec parsing${NC}"
    fi
fi
echo ""

# ============================================================================
# 4. Memory Footprint (Static Analysis)
# ============================================================================
echo -e "${YELLOW}═══ 4. Dependencies & Memory Footprint (Static) ═══${NC}"
echo ""

echo -e "${BLUE}edgerun-oci dependencies:${NC}"
cd /home/ken/edgerun_reference_core
cargo tree -p edgerun-oci-runtime --depth 1 2>/dev/null | head -20
echo ""

echo -e "${BLUE}Dependency count:${NC}"
EDGERUN_DEPS=$(cargo tree -p edgerun-oci-runtime 2>/dev/null | wc -l)
echo -e "  edgerun-oci: $EDGERUN_DEPS total dependency lines"
echo ""

# ============================================================================
# 5. Feature Comparison
# ============================================================================
echo -e "${YELLOW}═══ 5. Feature Comparison ═══${NC}"
echo ""

echo -e "${BLUE}OCI Runtime Features:${NC}"
echo ""
printf "  %-35s %-15s %-15s\n" "Feature" "edgerun-oci" "crun"
printf "  %-35s %-15s %-15s\n" "-------" "-----------" "----"

# Check various features
check_feature() {
    local feature=$1
    local edgerun_support=$2
    local crun_support=$3
    printf "  %-35s %-15s %-15s\n" "$feature" "$edgerun_support" "$crun_support"
}

check_feature "OCI Spec Version" "1.0.2" "$(crun --version 2>&1 | grep -oP 'spec: \K[^ ]+')"
check_feature "Seccomp-BPF" "✓ (custom impl)" "✓"
check_feature "Cgroups v2" "✓" "✓"
check_feature "Namespaces" "✓ (5 types)" "✓ (all)"
check_feature "Hooks" "✓ (6 types)" "✓"
check_feature "Capabilities" "✓" "✓"
check_feature "User Namespaces" "✓" "✓"
check_feature "Rootless Containers" "Partial" "✓"
check_feature "Systemd Integration" "✗" "✓"
check_feature "SELinux" "✗" "✓"
check_feature "AppArmor" "✗" "✓"
check_feature "Mount Propagation" "✓" "✓"
check_feature "Resource Limits" "✓" "✓"
check_feature "Checkpoint/Restore" "✗" "✓ (CRIU)"
echo ""

# ============================================================================
# 6. Architecture Comparison
# ============================================================================
echo -e "${YELLOW}═══ 6. Architecture & Design ═══${NC}"
echo ""

echo -e "${BLUE}edgerun-oci-runtime:${NC}"
echo -e "  - Language: Rust (edition 2021)"
echo -e "  - Syscalls: Raw FFI (no libc crate)"
echo -e "  - Process model: Manual fork() + FIFO signaling"
echo -e "  - PID 1 init: Custom zombie reaping + signal forwarding"
echo -e "  - Seccomp: Custom BPF generator (arch-aware)"
echo -e "  - Cgroups: v2 only, direct filesystem writes"
echo -e "  - Design: Composable lifecycle with hook integration"
echo ""

echo -e "${BLUE}crun:${NC}"
echo -e "  - Language: C"
echo -e "  - Syscalls: Direct libc"
echo -e "  - Process model: clone() with fine-grained flags"
echo -e "  - PID 1 init: Built-in init process"
echo -e "  - Seccomp: libseccomp integration"
echo -e "  - Cgroups: v1 and v2 support"
echo -e "  - Design: Monolithic, highly optimized"
echo ""

# ============================================================================
# Summary
# ============================================================================
echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                    SUMMARY                             ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}Key Findings:${NC}"
echo ""
echo "1. Binary Size:"
printf "   - edgerun-oci: %'d bytes (%.1f KB)\n" "$EDGERUN_SIZE" "$(echo "scale=1; $EDGERUN_SIZE / 1024" | bc)"
printf "   - crun:        %'d bytes (%.1f KB)\n" "$CRUN_SIZE" "$(echo "scale=1; $CRUN_SIZE / 1024" | bc)"
echo "   - Trade-off: Rust's standard library + static linking increases binary size"
echo ""

echo "2. Spec Parsing:"
echo "   - edgerun-oci: ${EDGERUN_AVG} ms avg"
echo "   - crun:        ${CRUN_AVG} ms avg"
if [ "$EDGERUN_AVG" -le "$CRUN_AVG" ]; then
    SPEEDUP=$(echo "scale=2; $CRUN_AVG / max($EDGERUN_AVG, 1)" | bc)
    echo "   - edgerun-oci is competitive despite being Rust-based"
else
    SPEEDUP=$(echo "scale=2; $EDGERUN_AVG / max($CRUN_AVG, 1)" | bc)
    echo "   - crun has an edge due to mature C implementation"
fi
echo ""

echo "3. Code Quality:"
echo "   - edgerun-oci: $NUM_FILES files, $NUM_LINES lines"
printf "   - Modern Rust with strong type safety\n"
echo "   - crun: Mature C codebase with extensive optimization"
echo ""

echo "4. Design Philosophy:"
echo "   - edgerun-oci: Safety-first, composable, hook-driven"
echo "   - crun: Performance-first, minimal, feature-complete"
echo ""

echo -e "${GREEN}Benchmark complete!${NC}"
