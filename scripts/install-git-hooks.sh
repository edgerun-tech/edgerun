#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push scripts/scan-secrets.sh

echo "installed git hooks:"
echo "  core.hooksPath=$(git config --get core.hooksPath)"
