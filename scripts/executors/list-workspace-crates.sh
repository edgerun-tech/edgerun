#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

find crates -mindepth 2 -maxdepth 2 -name Cargo.toml -print0 \
  | xargs -0 awk '
    /^name = "/ {
      gsub(/^name = "/, "", $0);
      gsub(/".*/, "", $0);
      print $0;
      nextfile;
    }
  ' \
  | LC_ALL=C sort -u
