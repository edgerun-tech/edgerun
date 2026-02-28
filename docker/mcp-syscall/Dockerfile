# SPDX-License-Identifier: Apache-2.0
FROM archlinux:base

RUN pacman -Syu --noconfirm --needed \
  go \
  gcc \
  pkgconf \
  webkit2gtk-4.1 \
  gtk3 \
  ca-certificates \
  && pacman -Scc --noconfirm

WORKDIR /workspace/mcp-syscall
ENTRYPOINT ["/bin/sh", "-lc", "go run ."]
