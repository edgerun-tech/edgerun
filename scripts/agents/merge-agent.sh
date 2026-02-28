#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage:" >&2
  echo "  scripts/agents/merge-agent.sh <RUN_DIR|PATCH_PATH>   # publish accepted diff event" >&2
  echo "  scripts/agents/merge-agent.sh <AGENT_BRANCH> [TARGET_BRANCH]" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARG1="$1"
TARGET_BRANCH="${2:-}"

cd "${ROOT_DIR}"

if [[ -d "${ARG1}" || -f "${ARG1}" ]]; then
  exec "${ROOT_DIR}/scripts/agents/apply-accepted-diff.sh" "${ARG1}"
fi

AGENT_BRANCH="${ARG1}"

if [[ -z "${TARGET_BRANCH}" ]]; then
  TARGET_BRANCH="$(git branch --show-current)"
fi

if [[ -z "${TARGET_BRANCH}" ]]; then
  echo "failed to resolve target branch" >&2
  exit 1
fi

if ! git show-ref --verify --quiet "refs/heads/${AGENT_BRANCH}"; then
  echo "agent branch not found: ${AGENT_BRANCH}" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree is dirty; commit or stash before merge" >&2
  exit 1
fi

current="$(git branch --show-current)"
if [[ "${current}" != "${TARGET_BRANCH}" ]]; then
  git checkout "${TARGET_BRANCH}"
fi

git merge --no-ff --no-edit "${AGENT_BRANCH}"

echo "merge complete: ${AGENT_BRANCH} -> ${TARGET_BRANCH}"
