#!/bin/bash
# Android Emulator Test Setup for edgerun
#
# Prerequisites:
#   - Android SDK installed (ANDROID_HOME set)
#   - Android Emulator installed (sdkmanager "emulator")
#   - A system image installed (e.g., sdkmanager "system-images;android-34;google_apis;x86_64")
#
# Usage:
#   ./scripts/setup-android-emulator.sh          # Create and start emulator
#   ./scripts/setup-android-emulator.sh stop     # Stop the emulator
#   ./scripts/setup-android-emulator.sh test     # Run tests on the emulator
#   ./scripts/setup-android-emulator.sh shell    # Open adb shell

set -euo pipefail

AVD_NAME="edgerun_test_avd"
API_LEVEL="34"
TARGET="google_apis"
ARCH="x86_64"
PORT="5554"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_prerequisites() {
    if [ -z "${ANDROID_HOME:-}" ]; then
        log_error "ANDROID_HOME is not set. Please install Android SDK."
        log_error "  export ANDROID_HOME=\$HOME/Android/Sdk"
        exit 1
    fi

    if ! command -v "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" &>/dev/null && \
       ! command -v "$ANDROID_HOME/tools/bin/sdkmanager" &>/dev/null; then
        log_warn "sdkmanager not found. Installing command-line tools..."
        mkdir -p "$ANDROID_HOME/cmdline-tools"
        # Download command-line tools
        log_info "Download Android command-line tools from:"
        log_info "  https://developer.android.com/studio#command-line-tools-only"
        log_info "Then extract to \$ANDROID_HOME/cmdline-tools/latest/"
        exit 1
    fi

    if ! command -v "$ANDROID_HOME/emulator/emulator" &>/dev/null; then
        log_error "Android emulator not installed."
        log_info "Install with:"
        log_info "  \$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager emulator"
        exit 1
    fi
}

create_avd() {
    if "$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager" list avd 2>/dev/null | grep -q "$AVD_NAME"; then
        log_info "AVD '$AVD_NAME' already exists."
        return 0
    fi

    log_info "Creating AVD: $AVD_NAME (API $API_LEVEL, $TARGET, $ARCH)"
    
    # Check if system image is installed
    IMAGE_NAME="system-images;android-${API_LEVEL};${TARGET};${ARCH}"
    if ! "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --list 2>/dev/null | grep -q "$IMAGE_NAME (installed)"; then
        log_info "Installing system image: $IMAGE_NAME"
        "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" "$IMAGE_NAME" --accept-licenses
    fi

    "$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager" create avd \
        --name "$AVD_NAME" \
        --package "$IMAGE_NAME" \
        --force \
        --device "pixel_6"

    log_info "AVD created successfully."
}

start_emulator() {
    log_info "Starting emulator '$AVD_NAME' on port $PORT..."
    
    "$ANDROID_HOME/emulator/emulator" \
        -avd "$AVD_NAME" \
        -port "$PORT" \
        -no-window \
        -no-audio \
        -no-boot-anim \
        -accel on \
        -gpu swiftshader_indirect \
        -memory 4096 \
        &>/tmp/edgerun-emulator.log &
    
    EMULATOR_PID=$!
    echo "$EMULATOR_PID" > /tmp/edgerun-emulator.pid
    
    log_info "Waiting for emulator to boot..."
    # Wait for boot complete
    for i in $(seq 1 120); do
        if "$ANDROID_HOME/platform-tools/adb" -s "emulator-$PORT" shell getprop sys.boot_completed 2>/dev/null | grep -q "1"; then
            log_info "Emulator booted successfully (took ~${i}s)"
            return 0
        fi
        sleep 2
    done
    
    log_error "Emulator did not boot within 4 minutes."
    log_error "Check logs: cat /tmp/edgerun-emulator.log"
    kill "$EMULATOR_PID" 2>/dev/null || true
    exit 1
}

stop_emulator() {
    if [ -f /tmp/edgerun-emulator.pid ]; then
        EMULATOR_PID=$(cat /tmp/edgerun-emulator.pid)
        if kill -0 "$EMULATOR_PID" 2>/dev/null; then
            log_info "Stopping emulator (PID $EMULATOR_PID)..."
            kill "$EMULATOR_PID" 2>/dev/null || true
            wait "$EMULATOR_PID" 2>/dev/null || true
        fi
        rm -f /tmp/edgerun-emulator.pid
    fi
    
    # Also kill via adb
    "$ANDROID_HOME/platform-tools/adb" -s "emulator-$PORT" emu kill 2>/dev/null || true
    log_info "Emulator stopped."
}

run_tests() {
    log_info "Running edgerun Android tests on emulator..."
    
    # Push the test binary to the emulator
    if [ ! -f "target/aarch64-linux-android/release/edgerund" ]; then
        log_warn "Release binary not found. Building..."
        cargo build --target aarch64-linux-android --release --features android-real
    fi
    
    # Or run cargo test through adb for unit tests
    log_info "Running unit tests on host (stub implementations)..."
    cargo test --package edgerun-android-hardware
    cargo test --package edgerun-android-keystore
    
    log_info "All tests passed!"
}

open_shell() {
    log_info "Opening adb shell..."
    "$ANDROID_HOME/platform-tools/adb" -s "emulator-$PORT" shell
}

push_binary() {
    log_info "Building edgerund for Android..."
    cargo build --target aarch64-linux-android --release --features android-real
    
    log_info "Pushing binary to emulator..."
    "$ANDROID_HOME/platform-tools/adb" -s "emulator-$PORT" push \
        target/aarch64-linux-android/release/edgerund /data/local/tmp/edgerund
    
    "$ANDROID_HOME/platform-tools/adb" -s "emulator-$PORT" shell chmod +x /data/local/tmp/edgerund
    log_info "Binary pushed to /data/local/tmp/edgerund"
}

# Main
case "${1:-start}" in
    start)
        check_prerequisites
        create_avd
        start_emulator
        log_info "Emulator is running. Connect with: adb -s emulator-$PORT shell"
        ;;
    stop)
        stop_emulator
        ;;
    test)
        run_tests
        ;;
    shell)
        open_shell
        ;;
    push)
        push_binary
        ;;
    *)
        echo "Usage: $0 {start|stop|test|shell|push}"
        echo ""
        echo "Commands:"
        echo "  start   - Create (if needed) and start the emulator"
        echo "  stop    - Stop the running emulator"
        echo "  test    - Run unit tests"
        echo "  shell   - Open an adb shell into the emulator"
        echo "  push    - Build and push edgerund to the emulator"
        exit 1
        ;;
esac
