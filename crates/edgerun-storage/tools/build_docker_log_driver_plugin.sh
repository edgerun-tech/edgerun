#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$CRATE_DIR/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/out/docker-log-driver-plugin}"
TARGET_DIR="${TARGET_DIR:-/var/cache/build/rust/target}"

BIN_SRC="$TARGET_DIR/release/docker_log_driver"
ROOTFS_DIR="$OUT_DIR/rootfs"
BIN_DST="$ROOTFS_DIR/usr/local/bin/docker_log_driver"
CONFIG_PATH="$OUT_DIR/config.json"
LIBC_SO="/usr/lib/libc.so.6"
LIBGCC_SO="/usr/lib/libgcc_s.so.1"
LD_SO="/usr/lib64/ld-linux-x86-64.so.2"

mkdir -p "$OUT_DIR"

cargo build --release -p edgerun-storage --bin docker_log_driver

rm -rf "$ROOTFS_DIR"
mkdir -p "$(dirname "$BIN_DST")"
install -m 0755 "$BIN_SRC" "$BIN_DST"
install -D -m 0755 "$LD_SO" "$ROOTFS_DIR/lib64/ld-linux-x86-64.so.2"
install -D -m 0644 "$LIBC_SO" "$ROOTFS_DIR/usr/lib/libc.so.6"
install -D -m 0644 "$LIBGCC_SO" "$ROOTFS_DIR/usr/lib/libgcc_s.so.1"

cat > "$CONFIG_PATH" <<'JSON'
{
  "description": "Edgerun storage-backed Docker log driver",
  "documentation": "https://github.com/edgerun/edgerun",
  "entrypoint": [
    "/usr/local/bin/docker_log_driver"
  ],
  "workdir": "/",
  "network": {
    "type": "host"
  },
  "interface": {
    "types": [
      "docker.logdriver/1.0"
    ],
    "socket": "e.sock"
  },
  "linux": {
    "capabilities": [],
    "allow-all-devices": false,
    "devices": null
  },
  "mounts": [
    {
      "name": "edgerun-log-data",
      "description": "Persistent log storage",
      "destination": "/var/lib/edgerun/docker-log-driver",
      "type": "bind",
      "source": "/var/lib/edgerun/docker-log-driver",
      "options": ["rbind", "rw"]
    }
  ],
  "env": [
    {
      "name": "DATA_DIR",
      "description": "Storage data directory",
      "settable": ["value"],
      "value": "/var/lib/edgerun/docker-log-driver"
    },
    {
      "name": "REPO_ID",
      "description": "VFS repo id for log stream",
      "settable": ["value"],
      "value": "docker-logs"
    },
    {
      "name": "BRANCH",
      "description": "Branch id for log stream",
      "settable": ["value"],
      "value": "main"
    },
    {
      "name": "SOCKET_PATH",
      "description": "Unix socket for Docker plugin API",
      "settable": ["value"],
      "value": "/run/docker/plugins/e.sock"
    },
    {
      "name": "DECLARED_BY",
      "description": "Declared-by actor",
      "settable": ["value"],
      "value": "docker_log_driver_plugin"
    },
    {
      "name": "PARTITION_PREFIX",
      "description": "Partition prefix",
      "settable": ["value"],
      "value": "docker"
    },
    {
      "name": "BATCH_LINES",
      "description": "Ingest flush batch size",
      "settable": ["value"],
      "value": "1000"
    },
    {
      "name": "REQUEST_MAX_BYTES",
      "description": "HTTP request body max bytes",
      "settable": ["value"],
      "value": "1048576"
    },
    {
      "name": "MAX_STREAM_BUFFER_BYTES",
      "description": "Per-stream decode buffer cap in bytes",
      "settable": ["value"],
      "value": "8388608"
    },
    {
      "name": "ENSURE_LOG_SOURCE",
      "description": "Write source import event at startup (1/0)",
      "settable": ["value"],
      "value": "1"
    }
  ]
}
JSON

printf 'plugin_bundle=%s\n' "$OUT_DIR"
printf 'plugin_config=%s\n' "$CONFIG_PATH"
printf 'plugin_rootfs_binary=%s\n' "$BIN_DST"
