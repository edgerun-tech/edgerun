#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_FILE="${1:-${ROOT_DIR}/out/swarm/crate-executors.stack.yml}"
RUNNER_SCRIPT="${ROOT_DIR}/scripts/executors/swarm-crate-runner.sh"
SWARM_NODE_ADDR="$(docker info --format '{{.Swarm.NodeAddr}}' 2>/dev/null || true)"
DEFAULT_NATS_URL="nats://127.0.0.1:4222"
if [[ -n "${SWARM_NODE_ADDR}" ]]; then
  DEFAULT_NATS_URL="nats://${SWARM_NODE_ADDR}:4222"
fi
NATS_URL="${EDGERUN_EVENTBUS_NATS_URL:-${DEFAULT_NATS_URL}}"
CODE_UPDATE_SUBJECT="${CODE_UPDATE_SUBJECT:-edgerun.code.updated}"
CONSTRAINT_KEY="${EDGERUN_EXECUTOR_NODE_LABEL_KEY:-edgerun.role}"
CONSTRAINT_VAL="${EDGERUN_EXECUTOR_NODE_LABEL_VALUE:-executor}"
CODE_REPO_URL="${EDGERUN_CODE_REPO_URL:-$(git -C "${ROOT_DIR}" config --get remote.origin.url || true)}"
CODE_REF="${EDGERUN_CODE_REF:-$(git -C "${ROOT_DIR}" rev-parse --abbrev-ref HEAD)}"
RUNNER_SHA="$(sha256sum "${RUNNER_SCRIPT}" | awk '{print substr($1,1,12)}')"
RUNNER_CONFIG_NAME="crate-executor-runner-${RUNNER_SHA}"

if [[ -z "${CODE_REPO_URL}" ]]; then
  echo "EDGERUN_CODE_REPO_URL is required (or configure git remote.origin.url)" >&2
  exit 1
fi

if [[ "${CODE_REPO_URL}" == git@github.com:* ]]; then
  CODE_REPO_URL="https://github.com/${CODE_REPO_URL#git@github.com:}"
fi
CODE_REPO_URL="${CODE_REPO_URL%.git}.git"

mkdir -p "$(dirname "${OUT_FILE}")"

cat > "${OUT_FILE}" <<YAML
# SPDX-License-Identifier: Apache-2.0
configs:
  ${RUNNER_CONFIG_NAME}:
    file: ${RUNNER_SCRIPT}

services:
YAML

while IFS= read -r crate; do
  safe="$(printf '%s' "$crate" | tr '[:upper:]' '[:lower:]' | tr -cd 'a-z0-9-_' | tr '_' '-')"
  cat >> "${OUT_FILE}" <<YAML
  crate-build-${safe}:
    image: rust:1.85-bookworm
    command: ["/bin/bash", "/opt/edgerun/swarm-crate-runner.sh"]
    environment:
      CRATE_NAME: "${crate}"
      EXECUTOR_MODE: "build"
      EDGERUN_EVENTBUS_NATS_URL: "${NATS_URL}"
      CODE_UPDATE_SUBJECT: "${CODE_UPDATE_SUBJECT}"
      EDGERUN_CODE_REPO_URL: "${CODE_REPO_URL}"
      EDGERUN_CODE_REF: "${CODE_REF}"
      NODE_ID_OVERRIDE: "{{.Node.Hostname}}"
    configs:
      - source: ${RUNNER_CONFIG_NAME}
        target: /opt/edgerun/swarm-crate-runner.sh
        mode: 0555
    deploy:
      mode: replicated
      replicas: 1
      placement:
        constraints:
          - node.labels.${CONSTRAINT_KEY} == ${CONSTRAINT_VAL}
      restart_policy:
        condition: on-failure

  crate-test-${safe}:
    image: rust:1.85-bookworm
    command: ["/bin/bash", "/opt/edgerun/swarm-crate-runner.sh"]
    environment:
      CRATE_NAME: "${crate}"
      EXECUTOR_MODE: "test"
      EDGERUN_EVENTBUS_NATS_URL: "${NATS_URL}"
      CODE_UPDATE_SUBJECT: "${CODE_UPDATE_SUBJECT}"
      EDGERUN_CODE_REPO_URL: "${CODE_REPO_URL}"
      EDGERUN_CODE_REF: "${CODE_REF}"
      NODE_ID_OVERRIDE: "{{.Node.Hostname}}"
    configs:
      - source: ${RUNNER_CONFIG_NAME}
        target: /opt/edgerun/swarm-crate-runner.sh
        mode: 0555
    deploy:
      mode: replicated
      replicas: 1
      placement:
        constraints:
          - node.labels.${CONSTRAINT_KEY} == ${CONSTRAINT_VAL}
      restart_policy:
        condition: on-failure

YAML
done < <("${ROOT_DIR}/scripts/executors/list-workspace-crates.sh")

echo "generated ${OUT_FILE}"
