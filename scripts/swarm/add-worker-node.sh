#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

WORKER_HOST="${1:-10.13.37.2}"
SSH_USER="${EDGERUN_SWARM_WORKER_SSH_USER:-root}"
SSH_PORT="${EDGERUN_SWARM_WORKER_SSH_PORT:-22}"
LABEL_KEY="${EDGERUN_SWARM_WORKER_LABEL_KEY:-edgerun.role}"
LABEL_VAL="${EDGERUN_SWARM_WORKER_LABEL_VAL:-executor}"
MANAGER_ADDR_OVERRIDE="${EDGERUN_SWARM_MANAGER_ADDR:-}"

state="$(docker info --format '{{.Swarm.LocalNodeState}}' 2>/dev/null || true)"
manager="$(docker info --format '{{.Swarm.ControlAvailable}}' 2>/dev/null || true)"
if [[ "${state}" != "active" || "${manager}" != "true" ]]; then
  echo "this node must be an active swarm manager" >&2
  exit 1
fi

token="$(docker swarm join-token -q worker)"
manager_addr="${MANAGER_ADDR_OVERRIDE}"
if [[ -z "${manager_addr}" ]]; then
  manager_addr="$(ip -4 route get "${WORKER_HOST}" 2>/dev/null | awk '/src/ {for(i=1;i<=NF;i++) if ($i=="src") {print $(i+1); exit}}')"
fi
if [[ -z "${manager_addr}" ]]; then
  manager_addr="$(docker info --format '{{.Swarm.NodeAddr}}')"
fi
join_cmd="docker swarm join --token ${token} ${manager_addr}:2377"

echo "joining worker ${WORKER_HOST} via ssh"
if ! ssh -p "${SSH_PORT}" -o BatchMode=yes -o StrictHostKeyChecking=accept-new "${SSH_USER}@${WORKER_HOST}" "command -v docker >/dev/null 2>&1"; then
  echo "remote node ${WORKER_HOST} is reachable but docker is not installed/on PATH" >&2
  echo "install docker on ${WORKER_HOST} and re-run: scripts/swarm/add-worker-node.sh ${WORKER_HOST}" >&2
  exit 1
fi
ssh -p "${SSH_PORT}" -o BatchMode=yes -o StrictHostKeyChecking=accept-new "${SSH_USER}@${WORKER_HOST}" "${join_cmd}"

sleep 2
node_id="$(docker node ls --format '{{.ID}} {{.Hostname}}' | awk -v host="${WORKER_HOST}" '$2==host {print $1; exit}')"
if [[ -z "${node_id}" ]]; then
  # fallback: match by IP in inspect address table
  node_id="$(docker node ls -q | while read -r nid; do
    addr="$(docker node inspect -f '{{.Status.Addr}}' "$nid" 2>/dev/null || true)"
    if [[ "$addr" == "${WORKER_HOST}" ]]; then
      echo "$nid"
      break
    fi
  done)"
fi

if [[ -z "${node_id}" ]]; then
  echo "worker joined but could not resolve node id for ${WORKER_HOST}" >&2
  docker node ls
  exit 1
fi

docker node update --label-add "${LABEL_KEY}=${LABEL_VAL}" "${node_id}" >/dev/null

echo "worker added: ${WORKER_HOST} (${node_id})"
echo "joined manager address: ${manager_addr}:2377"
echo "label set: ${LABEL_KEY}=${LABEL_VAL}"
docker node ls
