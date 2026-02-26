#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
EDGERUN_CFG_DIR="${HOME}/.config/edgerun"
WORKERS_CFG_DIR="${EDGERUN_CFG_DIR}/workers"
EDGERUN_STACK_PROFILE="${1:-${EDGERUN_STACK_PROFILE:-local}}"
WORKER_COUNT="${2:-3}"

if ! [[ "${WORKER_COUNT}" =~ ^[0-9]+$ ]] || [[ "${WORKER_COUNT}" -lt 1 ]]; then
  echo "ERR: worker_count must be a positive integer" >&2
  exit 1
fi

PROFILE_DIR="${ROOT_DIR}/scripts/systemd/env/profiles/${EDGERUN_STACK_PROFILE}"

if [[ ! -d "${PROFILE_DIR}" ]]; then
  echo "ERR: unknown stack profile '${EDGERUN_STACK_PROFILE}'" >&2
  echo "Available profiles:" >&2
  ls -1 "${ROOT_DIR}/scripts/systemd/env/profiles" 2>/dev/null || true
  exit 1
fi

mkdir -p "${SYSTEMD_USER_DIR}" "${EDGERUN_CFG_DIR}" "${WORKERS_CFG_DIR}"

fail() {
  echo "ERR: $*" >&2
  exit 1
}

warn() {
  echo "WARN: $*" >&2
}

has_token() {
  local file="$1"
  grep -Eq '__HOME__|__ROOT_DIR__|__PROFILE__' "${file}"
}

materialize_profile_env() {
  local source_file="$1"
  local target_file="$2"
  mkdir -p "$(dirname "${target_file}")"
  sed \
    -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
    -e "s|__HOME__|${HOME}|g" \
    "${source_file}" > "${target_file}"
  chmod 0644 "${target_file}"
}

read_env_value() {
  local source_file="$1"
  local key="$2"
  if [[ ! -f "${source_file}" ]]; then
    return 1
  fi
  sed -n "s/^${key}=//p" "${source_file}" | head -n1
}

effective_env_value() {
  local base_file="$1"
  local override_file="$2"
  local key="$3"
  local value
  value="$(read_env_value "${base_file}" "${key}" || true)"
  if [[ -n "${override_file}" && -f "${override_file}" ]]; then
    local override_value
    override_value="$(read_env_value "${override_file}" "${key}" || true)"
    if [[ -n "${override_value}" ]]; then
      value="${override_value}"
    fi
  fi
  if [[ -z "${value}" ]]; then
    return 1
  fi
  printf '%s' "${value}"
}

require() {
  local name="$1"
  local value="$2"
  if [[ -z "${value}" ]]; then
    fail "${name} is required for profile ${EDGERUN_STACK_PROFILE}"
  fi
}

require_bool() {
  local name="$1"
  local value="$2"
  if [[ "${value}" != "true" && "${value}" != "false" ]]; then
    fail "${name} must be true|false (got ${value})"
  fi
}

require_int() {
  local name="$1"
  local value="$2"
  local min="${3:-0}"
  if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
    fail "${name} must be an integer (got ${value})"
  fi
  if (( value < min )); then
    fail "${name} must be >= ${min} (got ${value})"
  fi
}

require_socket() {
  local name="$1"
  local value="$2"
  if [[ "${value}" == *:* ]]; then
    local host="${value%:*}"
    local port="${value##*:}"
    if [[ -z "${host}" || -z "${port}" ]]; then
      fail "${name} is not a valid host:port value (got ${value})"
    fi
    if ! [[ "${port}" =~ ^[0-9]+$ ]] || (( port < 1 || port > 65535 )); then
      fail "${name} has invalid port (got ${value})"
    fi
  else
    fail "${name} must include host:port (got ${value})"
  fi
}

require_http_url() {
  local name="$1"
  local value="$2"
  if [[ "${value}" != http://* && "${value}" != https://* ]]; then
    fail "${name} must be an http(s) URL (got ${value})"
  fi
}

require_base58_key() {
  local name="$1"
  local value="$2"
  if ! [[ "${value}" =~ ^[1-9A-HJ-NP-Za-km-z]{32,44}$ ]]; then
    fail "${name} is not a valid base58 value (got ${value})"
  fi
}

require_runtime_ids() {
  local name="$1"
  local value="$2"
  IFS=',' read -r -a ids <<< "${value}"
  for raw_id in "${ids[@]}"; do
    local id
    id="$(printf '%s' "${raw_id}" | tr -d '[:space:]')"
    if [[ -z "${id}" || ! "${id}" =~ ^[0-9a-fA-F]{64}$ ]]; then
      fail "${name} must contain comma-separated 64-char hex IDs (got ${value})"
    fi
  done
}

require_absent_tokens() {
  local file="$1"
  if has_token "${file}"; then
    fail "placeholder token found in rendered file ${file}; re-run install to expand __HOME__/__ROOT_DIR__"
  fi
}

validate_file_exists_when_enabled() {
  local name="$1"
  local value="$2"
  if [[ -n "${value}" && ! -f "${value}" ]]; then
    warn "${name} does not exist yet: ${value}"
  fi
}

validate_scheduler_env() {
  local base_file="$1"
  local override_file="${2:-}"

  if [[ -n "${override_file}" && -f "${override_file}" ]]; then
    require_absent_tokens "${override_file}"
  fi
  require_absent_tokens "${base_file}"

  local scheduler_addr scheduler_base_url scheduler_data_dir
  local scheduler_require_chain scheduler_require_policy scheduler_require_sig scheduler_require_attestation scheduler_require_quorum
  local chain_rpc_url chain_wallet chain_program_id
  scheduler_addr="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_ADDR)" || fail "missing EDGERUN_SCHEDULER_ADDR"
  scheduler_base_url="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_BASE_URL)" || fail "missing EDGERUN_SCHEDULER_BASE_URL"
  scheduler_data_dir="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_DATA_DIR)" || fail "missing EDGERUN_SCHEDULER_DATA_DIR"
  scheduler_require_chain="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT)" || fail "missing EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT"
  scheduler_require_policy="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION)" || fail "missing EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION"
  scheduler_require_sig="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES)" || fail "missing EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES"
  scheduler_require_attestation="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION)" || fail "missing EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION"
  scheduler_require_quorum="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION)" || fail "missing EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION"
  chain_rpc_url="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_CHAIN_RPC_URL)" || fail "missing EDGERUN_CHAIN_RPC_URL"
  chain_wallet="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_CHAIN_WALLET)" || fail "missing EDGERUN_CHAIN_WALLET"
  chain_program_id="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_CHAIN_PROGRAM_ID)" || fail "missing EDGERUN_CHAIN_PROGRAM_ID"

  require_socket "EDGERUN_SCHEDULER_ADDR" "${scheduler_addr}"
  require_http_url "EDGERUN_SCHEDULER_BASE_URL" "${scheduler_base_url}"
  require "EDGERUN_SCHEDULER_DATA_DIR" "${scheduler_data_dir}"
  require_bool "EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT" "${scheduler_require_chain}"
  require_bool "EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION" "${scheduler_require_policy}"
  require_bool "EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES" "${scheduler_require_sig}"
  require_bool "EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION" "${scheduler_require_attestation}"
  require_bool "EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION" "${scheduler_require_quorum}"
  require_http_url "EDGERUN_CHAIN_RPC_URL" "${chain_rpc_url}"
  require "EDGERUN_SCHEDULER_CHAIN_WALLET" "${chain_wallet}"
  if [[ "${chain_wallet}" == /* ]]; then
    validate_file_exists_when_enabled "EDGERUN_CHAIN_WALLET" "${chain_wallet}"
  else
    fail "EDGERUN_CHAIN_WALLET must be an absolute path"
  fi
  require "EDGERUN_SCHEDULER_CHAIN_PROGRAM_ID" "${chain_program_id}"
}

validate_worker_common_env() {
  local base_file="$1"
  local override_file="${2:-}"
  if [[ -n "${override_file}" && -f "${override_file}" ]]; then
    require_absent_tokens "${override_file}"
  fi
  require_absent_tokens "${base_file}"

  local scheduler_url worker_runtime_ids worker_max_concurrent worker_mem_bytes worker_chain_submit
  scheduler_url="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_SCHEDULER_URL)" || fail "missing EDGERUN_SCHEDULER_URL"
  worker_runtime_ids="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_WORKER_RUNTIME_IDS)" || fail "missing EDGERUN_WORKER_RUNTIME_IDS"
  worker_max_concurrent="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_WORKER_MAX_CONCURRENT)" || fail "missing EDGERUN_WORKER_MAX_CONCURRENT"
  worker_mem_bytes="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_WORKER_MEM_BYTES)" || fail "missing EDGERUN_WORKER_MEM_BYTES"
  worker_chain_submit="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED)" || fail "missing EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED"

  require_http_url "EDGERUN_SCHEDULER_URL" "${scheduler_url}"
  require_runtime_ids "EDGERUN_WORKER_RUNTIME_IDS" "${worker_runtime_ids}"
  require_int "EDGERUN_WORKER_MAX_CONCURRENT" "${worker_max_concurrent}" 1
  require_int "EDGERUN_WORKER_MEM_BYTES" "${worker_mem_bytes}" 1
  require_bool "EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED" "${worker_chain_submit}"
}

validate_worker_instance_env() {
  local base_file="$1"
  local override_file="${2:-}"
  if [[ -n "${override_file}" && -f "${override_file}" ]]; then
    require_absent_tokens "${override_file}"
  fi
  require_absent_tokens "${base_file}"

  local worker_pubkey worker_version
  worker_pubkey="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_WORKER_PUBKEY)" || fail "missing EDGERUN_WORKER_PUBKEY"
  worker_version="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_WORKER_VERSION)" || fail "missing EDGERUN_WORKER_VERSION"
  if [[ "${worker_pubkey}" == worker-demo-* ]]; then
    warn "EDGERUN_WORKER_PUBKEY is set to a demo value (${worker_pubkey})"
  else
    require_base58_key "EDGERUN_WORKER_PUBKEY" "${worker_pubkey}"
  fi
  require "EDGERUN_WORKER_VERSION" "${worker_version}"
}

validate_term_env() {
  local base_file="$1"
  local override_file="${2:-}"
  if [[ -n "${override_file}" && -f "${override_file}" ]]; then
    require_absent_tokens "${override_file}"
  fi
  require_absent_tokens "${base_file}"

  local hardware_mode term_addr term_public_base
  hardware_mode="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_HARDWARE_MODE)" || fail "missing EDGERUN_HARDWARE_MODE"
  term_addr="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_TERM_SERVER_ADDR)" || fail "missing EDGERUN_TERM_SERVER_ADDR"
  term_public_base="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_TERM_PUBLIC_BASE_URL)" || fail "missing EDGERUN_TERM_PUBLIC_BASE_URL"

  if [[ "${hardware_mode}" != "allow-software" && "${hardware_mode}" != "required" ]]; then
    fail "EDGERUN_HARDWARE_MODE must be allow-software|required (got ${hardware_mode})"
  fi
  require_socket "EDGERUN_TERM_SERVER_ADDR" "${term_addr}"
  require_http_url "EDGERUN_TERM_PUBLIC_BASE_URL" "${term_public_base}"
}

validate_cloudflared_term_env() {
  local base_file="$1"
  local override_file="${2:-}"
  if [[ -n "${override_file}" && -f "${override_file}" ]]; then
    require_absent_tokens "${override_file}"
  fi
  require_absent_tokens "${base_file}"

  local tunnel_id tunnel_host tunnel_port
  tunnel_id="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_TERM_TUNNEL_ID)" || fail "missing EDGERUN_TERM_TUNNEL_ID"
  tunnel_host="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_TERM_TUNNEL_HOSTNAME)" || fail "missing EDGERUN_TERM_TUNNEL_HOSTNAME"
  tunnel_port="$(effective_env_value "${base_file}" "${override_file}" EDGERUN_TERM_SERVER_PORT)" || fail "missing EDGERUN_TERM_SERVER_PORT"

  if ! [[ "${tunnel_id}" =~ ^[0-9a-f-]{36}$ ]]; then
    fail "EDGERUN_TERM_TUNNEL_ID must be a valid UUID format (got ${tunnel_id})"
  fi
  require "EDGERUN_CLOUDFLARED_TUNNEL_HOSTNAME" "${tunnel_host}"
  require_int "EDGERUN_TERM_SERVER_PORT" "${tunnel_port}" 1
}

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-scheduler.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-scheduler.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-scheduler.service"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-worker@.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-worker@.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-worker@.service"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-term-server.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-term-server.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-term-server.service"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/edgerun-cloudflared-term.service" \
  > "${SYSTEMD_USER_DIR}/edgerun-cloudflared-term.service"
chmod 0644 "${SYSTEMD_USER_DIR}/edgerun-cloudflared-term.service"

sed \
  -e "s|__ROOT_DIR__|${ROOT_DIR}|g" \
  -e "s|__HOME__|${HOME}|g" \
  "${ROOT_DIR}/scripts/systemd/user/solana-test-validator.service" \
  > "${SYSTEMD_USER_DIR}/solana-test-validator.service"
chmod 0644 "${SYSTEMD_USER_DIR}/solana-test-validator.service"

SCHEDULER_SOURCE="${PROFILE_DIR}/scheduler.env"
if [[ ! -f "${SCHEDULER_SOURCE}" ]]; then
  SCHEDULER_SOURCE="${ROOT_DIR}/scripts/systemd/env/scheduler.env.example"
fi
materialize_profile_env "${SCHEDULER_SOURCE}" "${EDGERUN_CFG_DIR}/scheduler.env"

WORKER_COMMON_SOURCE="${PROFILE_DIR}/worker-common.env"
if [[ ! -f "${WORKER_COMMON_SOURCE}" ]]; then
  WORKER_COMMON_SOURCE="${ROOT_DIR}/scripts/systemd/env/worker-common.env.example"
fi
materialize_profile_env "${WORKER_COMMON_SOURCE}" "${EDGERUN_CFG_DIR}/worker-common.env"

TERM_SERVER_SOURCE="${PROFILE_DIR}/term-server.env"
if [[ ! -f "${TERM_SERVER_SOURCE}" ]]; then
  TERM_SERVER_SOURCE="${ROOT_DIR}/scripts/systemd/env/term-server.env.example"
fi
materialize_profile_env "${TERM_SERVER_SOURCE}" "${EDGERUN_CFG_DIR}/term-server.env"

CLOUDFLARED_SOURCE="${PROFILE_DIR}/cloudflared-term.env"
if [[ ! -f "${CLOUDFLARED_SOURCE}" ]]; then
  CLOUDFLARED_SOURCE="${ROOT_DIR}/scripts/systemd/env/cloudflared-term.env.example"
fi
materialize_profile_env "${CLOUDFLARED_SOURCE}" "${EDGERUN_CFG_DIR}/cloudflared-term.env"

for i in $(seq 1 "${WORKER_COUNT}"); do
  target="${WORKERS_CFG_DIR}/${i}.env"
  if [[ -f "${PROFILE_DIR}/workers/${i}.env" ]]; then
    materialize_profile_env "${PROFILE_DIR}/workers/${i}.env" "${target}"
    continue
  fi
  if [[ -f "${PROFILE_DIR}/worker-instance.env.example" ]]; then
    mkdir -p "${WORKERS_CFG_DIR}"
    sed "s/worker-demo-1/worker-demo-${i}/" \
      "${PROFILE_DIR}/worker-instance.env.example" > "${target}"
  else
    sed "s/worker-demo-1/worker-demo-${i}/" \
      "${ROOT_DIR}/scripts/systemd/env/worker-instance.env.example" > "${target}"
  fi
  chmod 0644 "${target}"
done

validate_scheduler_env "${EDGERUN_CFG_DIR}/scheduler.env" "${EDGERUN_CFG_DIR}/scheduler.override.env"
validate_worker_common_env "${EDGERUN_CFG_DIR}/worker-common.env" "${EDGERUN_CFG_DIR}/worker-common.override.env"
validate_term_env "${EDGERUN_CFG_DIR}/term-server.env" "${EDGERUN_CFG_DIR}/term-server.override.env"
validate_cloudflared_term_env "${EDGERUN_CFG_DIR}/cloudflared-term.env" "${EDGERUN_CFG_DIR}/cloudflared-term.override.env"

for i in $(seq 1 "${WORKER_COUNT}"); do
  validate_worker_instance_env "${WORKERS_CFG_DIR}/${i}.env" "${WORKERS_CFG_DIR}/${i}.override.env"
done

systemctl --user daemon-reload

echo "Installed user services:"
echo "  ${SYSTEMD_USER_DIR}/edgerun-scheduler.service"
echo "  ${SYSTEMD_USER_DIR}/edgerun-worker@.service"
echo "  ${SYSTEMD_USER_DIR}/edgerun-term-server.service"
echo "  ${SYSTEMD_USER_DIR}/edgerun-cloudflared-term.service"
echo "  ${SYSTEMD_USER_DIR}/solana-test-validator.service"
echo
echo "Config directory:"
echo "  ${EDGERUN_CFG_DIR}"
echo "Active profile: ${EDGERUN_STACK_PROFILE}"
echo
echo "Validated profile:"
echo "  ${EDGERUN_STACK_PROFILE}"
echo
echo "Next:"
echo "  systemctl --user enable --now edgerun-scheduler.service"
echo "  systemctl --user enable --now edgerun-worker@1.service edgerun-worker@2.service edgerun-worker@3.service"
echo "  systemctl --user enable --now edgerun-term-server.service edgerun-cloudflared-term.service"
