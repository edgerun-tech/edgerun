#!/bin/bash
# Benchmark comparison: edgerun-oci-runtime vs crun
# Tests container lifecycle performance, resource usage, and binary characteristics

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ITERATIONS=${ITERATIONS:-10}
TEST_IMAGE="alpine:latest"
BENCH_DIR="/tmp/edgerun-bench-$$"
RESULTS_FILE="$BENCH_DIR/results.csv"

# Runtime paths
EDGERUN_OCI="/home/ken/edgerun_reference_core/target/release/edgerun-oci"
CRUN=$(which crun)

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up benchmark files...${NC}"
    rm -rf "$BENCH_DIR"
    # Kill any remaining containers
    sudo rm -rf /tmp/edgerun-bench-*
}

trap cleanup EXIT

# Setup benchmark directory
mkdir -p "$BENCH_DIR"
echo "runtime,metric,value,unit" > "$RESULTS_FILE"

# Helper functions
log_result() {
    local runtime=$1
    local metric=$2
    local value=$3
    local unit=$4
    echo "$runtime,$metric,$value,$unit" >> "$RESULTS_FILE"
    echo -e "  ${BLUE}$metric:${NC} $value $unit"
}

run_timed() {
    local cmd=$1
    local start_time end_time elapsed
    
    start_time=$(date +%s%N)
    eval "$cmd"
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    echo $elapsed
}

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if [ ! -f "$EDGERUN_OCI" ]; then
    echo -e "${RED}edgerun-oci not found. Building in release mode...${NC}"
    cd /home/ken/edgerun_reference_core
    cargo build --release -p edgerun-oci-runtime
fi

if ! command -v crun &> /dev/null; then
    echo -e "${RED}crun is not installed${NC}"
    exit 1
fi

# Check if we can run with appropriate privileges
if [ "$EUID" -ne 0 ]; then
    echo -e "${YELLOW}Running without root. Some tests may require sudo.${NC}"
    echo -e "${YELLOW}For full benchmarks, run with: sudo $0${NC}"
fi

echo -e "${GREEN}✓ Prerequisites checked${NC}\n"

# ============================================================================
# BENCHMARK 1: Binary Size and Build Metrics
# ============================================================================
echo -e "${YELLOW}═══ Binary Characteristics ═══${NC}"

# Binary size
EDGERUN_SIZE=$(stat -c%s "$EDGERUN_OCI" 2>/dev/null || echo "0")
CRUN_SIZE=$(stat -c%s "$CRUN")

log_result "edgerun-oci" "binary_size" "$EDGERUN_SIZE" "bytes"
log_result "crun" "binary_size" "$CRUN_SIZE" "bytes"

# Binary size comparison
if [ "$EDGERUN_SIZE" -gt 0 ] && [ "$CRUN_SIZE" -gt 0 ]; then
    RATIO=$(echo "scale=2; $EDGERUN_SIZE / $CRUN_SIZE" | bc)
    echo -e "  ${BLUE}Size ratio (edgerun/crun):${NC} ${RATIO}x"
fi

# Check if edgerun-oci is statically or dynamically linked
echo -e "\n${YELLOW}Checking binary linking...${NC}"
file "$EDGERUN_OCI" 2>/dev/null || echo "Cannot determine edgerun-oci file type"
file "$CRUN" 2>/dev/null || echo "Cannot determine crun file type"

echo ""

# ============================================================================
# BENCHMARK 2: OCI Spec Parsing Performance
# ============================================================================
echo -e "${YELLOW}═══ OCI Spec Parsing Performance ═══${NC}"

# Create a standard OCI runtime spec
cat > "$BENCH_DIR/config.json" << 'EOF'
{
    "ociVersion": "1.0.2",
    "process": {
        "terminal": false,
        "user": {
            "uid": 0,
            "gid": 0
        },
        "args": [
            "/bin/sh",
            "-c",
            "echo hello && sleep 1"
        ],
        "env": [
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            "TERM=xterm"
        ],
        "cwd": "/",
        "capabilities": {
            "bounding": [
                "CAP_AUDIT_WRITE",
                "CAP_KILL",
                "CAP_NET_BIND_SERVICE"
            ],
            "effective": [
                "CAP_AUDIT_WRITE",
                "CAP_KILL",
                "CAP_NET_BIND_SERVICE"
            ],
            "inheritable": [
                "CAP_AUDIT_WRITE",
                "CAP_KILL",
                "CAP_NET_BIND_SERVICE"
            ],
            "permitted": [
                "CAP_AUDIT_WRITE",
                "CAP_KILL",
                "CAP_NET_BIND_SERVICE"
            ],
            "ambient": [
                "CAP_AUDIT_WRITE",
                "CAP_KILL",
                "CAP_NET_BIND_SERVICE"
            ]
        },
        "rlimits": [
            {
                "type": "RLIMIT_NOFILE",
                "hard": 1024,
                "soft": 1024
            }
        ],
        "noNewPrivileges": true
    },
    "root": {
        "path": "rootfs",
        "readonly": true
    },
    "hostname": "test-container",
    "mounts": [
        {
            "destination": "/proc",
            "type": "proc",
            "source": "proc"
        },
        {
            "destination": "/dev",
            "type": "tmpfs",
            "source": "tmpfs",
            "options": [
                "nosuid",
                "strictatime",
                "mode=755",
                "size=65536k"
            ]
        },
        {
            "destination": "/sys",
            "type": "sysfs",
            "source": "sysfs",
            "options": [
                "nosuid",
                "noexec",
                "nodev",
                "ro"
            ]
        }
    ],
    "linux": {
        "namespaces": [
            {
                "type": "pid"
            },
            {
                "type": "network"
            },
            {
                "type": "ipc"
            },
            {
                "type": "uts"
            },
            {
                "type": "mount"
            }
        ],
        "maskedPaths": [
            "/proc/kcore",
            "/proc/latency_stats",
            "/proc/timer_list",
            "/proc/timer_stats",
            "/proc/sched_debug"
        ],
        "readonlyPaths": [
            "/proc/asound",
            "/proc/bus",
            "/proc/fs",
            "/proc/irq",
            "/proc/sys",
            "/proc/sysrq-trigger"
        ]
    }
}
EOF

# Create minimal rootfs
mkdir -p "$BENCH_DIR/rootfs"

# Benchmark OCI spec parsing (edgerun-oci)
echo -e "${BLUE}Testing edgerun-oci spec parsing...${NC}"
EDGERUN_PARSE_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s%N)
    # Just validate/parse the spec (create command stops at container setup)
    timeout 5 "$EDGERUN_OCI" create --bundle "$BENCH_DIR" test_parse_$i 2>/dev/null || true
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    EDGERUN_PARSE_TIMES+=($elapsed)
    # Clean up
    timeout 5 "$EDGERUN_OCI" delete --force test_parse_$i 2>/dev/null || true
done

# Calculate average
EDGERUN_PARSE_AVG=$(printf '%s\n' "${EDGERUN_PARSE_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
log_result "edgerun-oci" "spec_parse_time" "$EDGERUN_PARSE_AVG" "ms"

# Benchmark OCI spec parsing (crun)
echo -e "${BLUE}Testing crun spec parsing...${NC}"
CRUN_PARSE_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s%N)
    timeout 5 "$CRUN" create test_parse_$i --bundle "$BENCH_DIR" 2>/dev/null || true
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    CRUN_PARSE_TIMES+=($elapsed)
    # Clean up
    timeout 5 "$CRUN" delete --force test_parse_$i 2>/dev/null || true
done

CRUN_PARSE_AVG=$(printf '%s\n' "${CRUN_PARSE_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
log_result "crun" "spec_parse_time" "$CRUN_PARSE_AVG" "ms"

echo ""

# ============================================================================
# BENCHMARK 3: Container Startup Time (create + start)
# ============================================================================
echo -e "${YELLOW}═══ Container Startup Time ═══${NC}"

# Create a simple OCI bundle for testing
setup_bundle() {
    local bundle_dir=$1
    
    # Create rootfs with basic utilities
    mkdir -p "$bundle_dir/rootfs"
    
    # Copy basic binaries and libraries (minimal rootfs)
    # For simplicity, use the host's /bin/sh
    cp /bin/sh "$bundle_dir/rootfs/" 2>/dev/null || ln -s /bin/sh "$bundle_dir/rootfs/"
    
    # Create minimal config
    cat > "$bundle_dir/config.json" << 'BUNDLE_EOF'
{
    "ociVersion": "1.0.2",
    "process": {
        "terminal": false,
        "user": { "uid": 0, "gid": 0 },
        "args": [ "/bin/sh", "-c", "echo test" ],
        "env": [ "PATH=/usr/bin:/bin", "TERM=xterm" ],
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
BUNDLE_EOF
}

# Test with edgerun-oci
echo -e "${BLUE}Testing edgerun-oci startup (create + start)...${NC}"
EDGERUN_START_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    BUNDLE_DIR="$BENCH_DIR/bundle_$i"
    mkdir -p "$BUNDLE_DIR"
    setup_bundle "$BUNDLE_DIR"
    
    start_time=$(date +%s%N)
    # Create container
    timeout 10 "$EDGERUN_OCI" create --bundle "$BUNDLE_DIR" "test_start_$i" 2>/dev/null
    # Start container
    timeout 10 "$EDGERUN_OCI" start "test_start_$i" 2>/dev/null
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    EDGERUN_START_TIMES+=($elapsed)
    
    # Clean up
    timeout 5 "$EDGERUN_OCI" delete --force "test_start_$i" 2>/dev/null || true
    rm -rf "$BUNDLE_DIR"
done

EDGERUN_START_AVG=$(printf '%s\n' "${EDGERUN_START_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
EDGERUN_START_MIN=$(printf '%s\n' "${EDGERUN_START_TIMES[@]}" | sort -n | head -1)
EDGERUN_START_MAX=$(printf '%s\n' "${EDGERUN_START_TIMES[@]}" | sort -n | tail -1)
log_result "edgerun-oci" "startup_time_avg" "$EDGERUN_START_AVG" "ms"
log_result "edgerun-oci" "startup_time_min" "$EDGERUN_START_MIN" "ms"
log_result "edgerun-oci" "startup_time_max" "$EDGERUN_START_MAX" "ms"

# Test with crun
echo -e "${BLUE}Testing crun startup (create + start)...${NC}"
CRUN_START_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    BUNDLE_DIR="$BENCH_DIR/bundle_$i"
    mkdir -p "$BUNDLE_DIR"
    setup_bundle "$BUNDLE_DIR"
    
    start_time=$(date +%s%N)
    # Create container
    timeout 10 "$CRUN" create "test_start_$i" --bundle "$BUNDLE_DIR" 2>/dev/null
    # Start container
    timeout 10 "$CRUN" start "test_start_$i" 2>/dev/null
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    CRUN_START_TIMES+=($elapsed)
    
    # Clean up
    timeout 5 "$CRUN" delete --force "test_start_$i" 2>/dev/null || true
    rm -rf "$BUNDLE_DIR"
done

CRUN_START_AVG=$(printf '%s\n' "${CRUN_START_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
CRUN_START_MIN=$(printf '%s\n' "${CRUN_START_TIMES[@]}" | sort -n | head -1)
CRUN_START_MAX=$(printf '%s\n' "${CRUN_START_TIMES[@]}" | sort -n | tail -1)
log_result "crun" "startup_time_avg" "$CRUN_START_AVG" "ms"
log_result "crun" "startup_time_min" "$CRUN_START_MIN" "ms"
log_result "crun" "startup_time_max" "$CRUN_START_MAX" "ms"

# Calculate speedup
if [ "$EDGERUN_START_AVG" -gt 0 ] && [ "$CRUN_START_AVG" -gt 0 ]; then
    if [ "$EDGERUN_START_AVG" -lt "$CRUN_START_AVG" ]; then
        SPEEDUP=$(echo "scale=2; $CRUN_START_AVG / $EDGERUN_START_AVG" | bc)
        echo -e "  ${GREEN}edgerun-oci is ${SPEEDUP}x faster than crun${NC}"
    else
        SPEEDUP=$(echo "scale=2; $EDGERUN_START_AVG / $CRUN_START_AVG" | bc)
        echo -e "  ${YELLOW}crun is ${SPEEDUP}x faster than edgerun-oci${NC}"
    fi
fi

echo ""

# ============================================================================
# BENCHMARK 4: Container Teardown Time (delete)
# ============================================================================
echo -e "${YELLOW}═══ Container Teardown Time ═══${NC}"

# Test with edgerun-oci
echo -e "${BLUE}Testing edgerun-oci teardown...${NC}"
EDGERUN_DELETE_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    BUNDLE_DIR="$BENCH_DIR/bundle_$i"
    mkdir -p "$BUNDLE_DIR"
    setup_bundle "$BUNDLE_DIR"
    
    # Create and start container
    "$EDGERUN_OCI" create --bundle "$BUNDLE_DIR" "test_del_$i" 2>/dev/null
    "$EDGERUN_OCI" start "test_del_$i" 2>/dev/null
    sleep 0.1  # Let it run briefly
    
    # Measure delete time
    start_time=$(date +%s%N)
    timeout 10 "$EDGERUN_OCI" delete --force "test_del_$i" 2>/dev/null
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    EDGERUN_DELETE_TIMES+=($elapsed)
    
    rm -rf "$BUNDLE_DIR"
done

EDGERUN_DELETE_AVG=$(printf '%s\n' "${EDGERUN_DELETE_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
log_result "edgerun-oci" "teardown_time_avg" "$EDGERUN_DELETE_AVG" "ms"

# Test with crun
echo -e "${BLUE}Testing crun teardown...${NC}"
CRUN_DELETE_TIMES=()
for i in $(seq 1 $ITERATIONS); do
    BUNDLE_DIR="$BENCH_DIR/bundle_$i"
    mkdir -p "$BUNDLE_DIR"
    setup_bundle "$BUNDLE_DIR"
    
    # Create and start container
    "$CRUN" create "test_del_$i" --bundle "$BUNDLE_DIR" 2>/dev/null
    "$CRUN" start "test_del_$i" 2>/dev/null
    sleep 0.1
    
    # Measure delete time
    start_time=$(date +%s%N)
    timeout 10 "$CRUN" delete --force "test_del_$i" 2>/dev/null
    end_time=$(date +%s%N)
    elapsed=$(( (end_time - start_time) / 1000000 ))
    CRUN_DELETE_TIMES+=($elapsed)
    
    rm -rf "$BUNDLE_DIR"
done

CRUN_DELETE_AVG=$(printf '%s\n' "${CRUN_DELETE_TIMES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
log_result "crun" "teardown_time_avg" "$CRUN_DELETE_AVG" "ms"

echo ""

# ============================================================================
# BENCHMARK 5: Memory Overhead (RSS of runtime processes)
# ============================================================================
echo -e "${YELLOW}═══ Memory Overhead ═══${NC}"

# This requires running containers for a bit and measuring runtime memory

# Test with edgerun-oci
echo -e "${BLUE}Testing edgerun-oci memory usage...${NC}"
EDGERUN_MEM_SAMPLES=()
for i in $(seq 1 $ITERATIONS); do
    BUNDLE_DIR="$BENCH_DIR/bundle_$i"
    mkdir -p "$BUNDLE_DIR"
    
    # Create a longer-running container
    cat > "$BUNDLE_DIR/config.json" << 'MEM_EOF'
{
    "ociVersion": "1.0.2",
    "process": {
        "terminal": false,
        "user": { "uid": 0, "gid": 0 },
        "args": [ "/bin/sh", "-c", "sleep 10" ],
        "env": [ "PATH=/usr/bin:/bin" ],
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
MEM_EOF
    
    setup_bundle "$BUNDLE_DIR"
    
    # Start container
    "$EDGERUN_OCI" create --bundle "$BUNDLE_DIR" "test_mem_$i" 2>/dev/null
    "$EDGERUN_OCI" start "test_mem_$i" 2>/dev/null
    sleep 0.5  # Let it stabilize
    
    # Measure memory of container processes
    # This is approximate - we're looking at the runtime's overhead
    MEM_USAGE=$(ps aux | grep -E "(edgerun|test_mem)" | grep -v grep | awk '{sum += $6} END {print sum}')
    if [ -n "$MEM_USAGE" ] && [ "$MEM_USAGE" -gt 0 ]; then
        EDGERUN_MEM_SAMPLES+=($MEM_USAGE)
    fi
    
    # Cleanup
    "$EDGERUN_OCI" delete --force "test_mem_$i" 2>/dev/null || true
    rm -rf "$BUNDLE_DIR"
done

if [ ${#EDGERUN_MEM_SAMPLES[@]} -gt 0 ]; then
    EDGERUN_MEM_AVG=$(printf '%s\n' "${EDGERUN_MEM_SAMPLES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
    log_result "edgerun-oci" "memory_overhead_avg" "$EDGERUN_MEM_AVG" "KB"
fi

# Test with crun
echo -e "${BLUE}Testing crun memory usage...${NC}"
CRUN_MEM_SAMPLES=()
for i in $(seq 1 $ITERATIONS); do
    BUNDLE_DIR="$BENCH_DIR/bundle_$i"
    mkdir -p "$BUNDLE_DIR"
    
    cat > "$BUNDLE_DIR/config.json" << 'MEM_EOF'
{
    "ociVersion": "1.0.2",
    "process": {
        "terminal": false,
        "user": { "uid": 0, "gid": 0 },
        "args": [ "/bin/sh", "-c", "sleep 10" ],
        "env": [ "PATH=/usr/bin:/bin" ],
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
MEM_EOF
    
    setup_bundle "$BUNDLE_DIR"
    
    "$CRUN" create "test_mem_$i" --bundle "$BUNDLE_DIR" 2>/dev/null
    "$CRUN" start "test_mem_$i" 2>/dev/null
    sleep 0.5
    
    MEM_USAGE=$(ps aux | grep -E "(crun|test_mem)" | grep -v grep | awk '{sum += $6} END {print sum}')
    if [ -n "$MEM_USAGE" ] && [ "$MEM_USAGE" -gt 0 ]; then
        CRUN_MEM_SAMPLES+=($MEM_USAGE)
    fi
    
    "$CRUN" delete --force "test_mem_$i" 2>/dev/null || true
    rm -rf "$BUNDLE_DIR"
done

if [ ${#CRUN_MEM_SAMPLES[@]} -gt 0 ]; then
    CRUN_MEM_AVG=$(printf '%s\n' "${CRUN_MEM_SAMPLES[@]}" | awk '{ sum += $1; n++ } END { if (n > 0) printf "%.0f", sum / n; else print 0 }')
    log_result "crun" "memory_overhead_avg" "$CRUN_MEM_AVG" "KB"
fi

echo ""

# ============================================================================
# SUMMARY
# ============================================================================
echo -e "${YELLOW}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${YELLOW}║              BENCHMARK SUMMARY                          ║${NC}"
echo -e "${YELLOW}╚══════════════════════════════════════════════════════════╝${NC}\n"

echo -e "${BLUE}Iterations per test:${NC} $ITERATIONS"
echo -e "${BLUE}Timestamp:${NC} $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
echo ""

echo -e "${GREEN}═══ Results ═══${NC}"
echo ""
echo "Results saved to: $RESULTS_FILE"
echo ""

# Create a formatted table
printf "%-20s %-25s %-15s %-10s\n" "Runtime" "Metric" "Value" "Unit"
printf "%-20s %-25s %-15s %-10s\n" "-------" "------" "-----" "----"

# Skip header line, display results
tail -n +2 "$RESULTS_FILE" | while IFS=',' read -r runtime metric value unit; do
    printf "%-20s %-25s %-15s %-10s\n" "$runtime" "$metric" "$value" "$unit"
done

echo ""
echo -e "${GREEN}═══ Detailed CSV ═══${NC}"
echo "Full results available in: $RESULTS_FILE"
cat "$RESULTS_FILE"

echo ""
echo -e "${GREEN}Benchmark complete!${NC}"
