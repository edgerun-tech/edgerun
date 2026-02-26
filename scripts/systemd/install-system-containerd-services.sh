#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

ENABLE_SERVICES="${ENABLE_SERVICES:-1}"
RESTART_CONTAINERD="${RESTART_CONTAINERD:-0}"
INSTALL_BINARIES="${INSTALL_BINARIES:-1}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "This script must run as root."
  exit 1
fi

if [[ "${INSTALL_BINARIES}" == "1" ]]; then
  export CARGO_TARGET_DIR="${ROOT_DIR}/out/target"
  cargo build --locked --release \
    -p edgerun-containerd-shim \
    -p edgerun-snapshotter

  install -Dm0755 "${CARGO_TARGET_DIR}/release/containerd-shim-edgerun-v1" /usr/lib/edgerun/containerd-shim-edgerun-backend
  install -Dm0755 "${CARGO_TARGET_DIR}/release/containerd-shim-edgerun-v2" /usr/bin/containerd-shim-edgerun-v2
  install -Dm0755 "${CARGO_TARGET_DIR}/release/containerd-shim-edgerun-v2" /usr/bin/containerd-shim-edgerun-v1
  install -Dm0755 "${CARGO_TARGET_DIR}/release/edgerun-snapshotterd" /usr/bin/edgerun-snapshotterd
fi

install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/edgerun-snapshotter.service" /etc/systemd/system/edgerun-snapshotter.service
install -Dm0644 "${ROOT_DIR}/scripts/systemd/system/edgerun-shim-backend.service" /etc/systemd/system/edgerun-shim-backend.service
install -Dm0644 "${ROOT_DIR}/config/containerd/edgerun-runtime-snippet.toml" /etc/containerd/edgerun-runtime-snippet.toml

mkdir -p /var/lib/edgerun/snapshotter /run/edgerun-snapshotter /run/edgerun-shim

systemctl daemon-reload

if [[ "${ENABLE_SERVICES}" == "1" ]]; then
  systemctl enable --now edgerun-snapshotter.service edgerun-shim-backend.service
else
  echo "Units installed. Start manually with:"
  echo "  systemctl enable --now edgerun-snapshotter.service edgerun-shim-backend.service"
fi

if [[ "${RESTART_CONTAINERD}" == "1" ]]; then
  systemctl restart containerd.service
fi

echo
echo "Installed:"
echo "  /usr/lib/edgerun/containerd-shim-edgerun-backend"
echo "  /usr/bin/containerd-shim-edgerun-v2"
echo "  /usr/bin/containerd-shim-edgerun-v1 (shim compatibility alias)"
echo "  /usr/bin/edgerun-snapshotterd"
echo "  /etc/systemd/system/edgerun-snapshotter.service"
echo "  /etc/systemd/system/edgerun-shim-backend.service"
echo "  /etc/containerd/edgerun-runtime-snippet.toml"
echo
echo "Next:"
echo "  1) Merge /etc/containerd/edgerun-runtime-snippet.toml into /etc/containerd/config.toml (or import it)."
echo "  2) Restart containerd when config is applied."
