#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

BIN_URL_DEFAULT="https://downloads.edgerun.tech/edgerun-node-manager/linux-amd64/edgerun-node-manager"
BIN_PATH_DEFAULT="/usr/local/bin/edgerun-node-manager"
SERVICE_NAME_DEFAULT="edgerun-node-manager.service"
SERVICE_PATH_DEFAULT="/etc/systemd/system/${SERVICE_NAME_DEFAULT}"
BRIDGE_LISTEN_DEFAULT="127.0.0.1:7777"

BIN_URL="${EDGERUN_NODE_MANAGER_URL:-$BIN_URL_DEFAULT}"
BIN_PATH="${EDGERUN_NODE_MANAGER_BIN_PATH:-$BIN_PATH_DEFAULT}"
SERVICE_NAME="$SERVICE_NAME_DEFAULT"
SERVICE_PATH="$SERVICE_PATH_DEFAULT"
BRIDGE_LISTEN="$BRIDGE_LISTEN_DEFAULT"
INSTALL_SERVICE=1

usage() {
  cat <<USAGE
Usage: $0 [options]

Options:
  --bin-url <url>             Binary URL (default: ${BIN_URL_DEFAULT})
  --bin-path <path>           Binary install path (default: ${BIN_PATH_DEFAULT})
  --bridge-listen <addr>      Bridge listen address (default: ${BRIDGE_LISTEN_DEFAULT})
  --service-name <name>       Service name (default: ${SERVICE_NAME_DEFAULT})
  --service-path <path>       Service install path (default: ${SERVICE_PATH_DEFAULT})
  --skip-service              Install binary only
  -h, --help                  Show help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin-url)
      BIN_URL="${2:-}"
      shift 2
      ;;
    --bin-path)
      BIN_PATH="${2:-}"
      shift 2
      ;;
    --bridge-listen)
      BRIDGE_LISTEN="${2:-}"
      shift 2
      ;;
    --service-name)
      SERVICE_NAME="${2:-}"
      shift 2
      ;;
    --service-path)
      SERVICE_PATH="${2:-}"
      shift 2
      ;;
    --skip-service)
      INSTALL_SERVICE=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$BIN_URL" || -z "$BIN_PATH" || -z "$BRIDGE_LISTEN" ]]; then
  echo "Invalid empty argument value" >&2
  exit 1
fi

if [[ "$BRIDGE_LISTEN" != 127.0.0.1:* && "$BRIDGE_LISTEN" != localhost:* ]]; then
  echo "Refusing non-loopback bridge listen address: $BRIDGE_LISTEN" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_TEMPLATE="${ROOT_DIR}/scripts/systemd/system/edgerun-node-manager.service"

if [[ ! -f "$SERVICE_TEMPLATE" ]]; then
  echo "Missing service template: $SERVICE_TEMPLATE" >&2
  exit 1
fi

if [[ $EUID -eq 0 ]]; then
  SUDO=()
else
  if command -v sudo >/dev/null 2>&1; then
    SUDO=(sudo)
  else
    echo "This installer needs root privileges (run as root or install sudo)." >&2
    exit 1
  fi
fi

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

TMP_BIN="${TMP_DIR}/edgerun-node-manager"

echo "Downloading edgerun-node-manager from: ${BIN_URL}"
curl -fsSL "$BIN_URL" -o "$TMP_BIN"
chmod +x "$TMP_BIN"

BIN_DIR="$(dirname "$BIN_PATH")"
"${SUDO[@]}" mkdir -p "$BIN_DIR"
"${SUDO[@]}" install -m 0755 "$TMP_BIN" "$BIN_PATH"

echo "Installed binary: ${BIN_PATH}"

if [[ "$INSTALL_SERVICE" -eq 1 ]]; then
  TMP_SERVICE="${TMP_DIR}/${SERVICE_NAME}"
  sed \
    -e "s|@BINARY@|${BIN_PATH}|g" \
    -e "s|@BRIDGE_LISTEN@|${BRIDGE_LISTEN}|g" \
    "$SERVICE_TEMPLATE" > "$TMP_SERVICE"

  "${SUDO[@]}" mkdir -p "$(dirname "$SERVICE_PATH")"
  "${SUDO[@]}" install -m 0644 "$TMP_SERVICE" "$SERVICE_PATH"

  "${SUDO[@]}" systemctl daemon-reload

  echo "Installed service: ${SERVICE_PATH}"
  echo
  echo "Next steps:"
  echo "  ${BIN_PATH} tunnel-connect --relay-control-base https://relay.edgerun.tech --pairing-code \"<PAIRING_CODE>\""
  echo "  ${SUDO[*]} systemctl enable --now ${SERVICE_NAME}"
  echo "  ${SUDO[*]} systemctl --no-pager --full status ${SERVICE_NAME}"
else
  echo
  echo "Service install skipped. Run manually:"
  echo "  ${BIN_PATH} run --local-bridge-listen ${BRIDGE_LISTEN}"
fi
