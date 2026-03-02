# SPDX-License-Identifier: Apache-2.0
FROM archlinux:base

# Runtime-only image for the prebuilt host binary.
RUN pacman -Syu --noconfirm --needed \
  git \
  webkit2gtk \
  gtk3 \
  ca-certificates \
  && pacman -Scc --noconfirm

WORKDIR /workspace/mcp-syscall
COPY docker/mcp-syscall/bin/mcp-syscall /opt/mcp-syscall/mcp-syscall
RUN chmod 0755 /opt/mcp-syscall/mcp-syscall

ENTRYPOINT ["/bin/sh", "-lc", "if [ -x /workspace/mcp-syscall/mcp-syscall ]; then exec /workspace/mcp-syscall/mcp-syscall; fi; exec /opt/mcp-syscall/mcp-syscall"]
