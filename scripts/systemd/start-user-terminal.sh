#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROFILE="${1:-${EDGERUN_STACK_PROFILE:-local}}"

enable_user_linger() {
  if ! command -v loginctl >/dev/null 2>&1; then
    echo "WARN: loginctl unavailable; boot-start cannot be verified automatically."
    return 0
  fi

  if [[ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null)" == "yes" ]]; then
    return 0
  fi

  echo "Enabling user lingering so services continue across reboot..."
  if ! loginctl enable-linger "$USER"; then
    echo "ERR: failed to enable user lingering for $USER."
    echo "Manual fix: sudo loginctl enable-linger \"$USER\""
    exit 1
  fi
}

"${ROOT_DIR}/scripts/systemd/install-user-services.sh" "${PROFILE}" 3
enable_user_linger

systemctl --user enable --now edgerun-term-server.service
systemctl --user enable --now edgerun-cloudflared-term.service

echo "Terminal stack started."
echo "Check:"
echo "  systemctl --user --no-pager --full status edgerun-term-server.service edgerun-cloudflared-term.service"
echo "  journalctl --user -u edgerun-term-server.service -f"
echo "  journalctl --user -u edgerun-cloudflared-term.service -f"
