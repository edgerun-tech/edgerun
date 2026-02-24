#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
required_fragment="${2:-$repo_root/config/nodeos-kernel-tpm.config}"
config_arg="${1:-}"

if [[ ! -f "$required_fragment" ]]; then
  echo "required config fragment missing: $required_fragment" >&2
  exit 1
fi

resolve_config() {
  if [[ -n "$config_arg" ]]; then
    echo "$config_arg"
    return 0
  fi
  if [[ -r /proc/config.gz ]]; then
    echo "/proc/config.gz"
    return 0
  fi
  local uname_cfg="/boot/config-$(uname -r)"
  if [[ -r "$uname_cfg" ]]; then
    echo "$uname_cfg"
    return 0
  fi
  return 1
}

config_path="$(resolve_config || true)"
if [[ -z "$config_path" ]]; then
  cat >&2 <<ERR
unable to resolve kernel config source.
provide explicit path: $0 <path-to-.config-or-config.gz>
ERR
  exit 1
fi

read_config() {
  local p="$1"
  if [[ "$p" == *.gz ]] || [[ "$p" == "/proc/config.gz" ]]; then
    zcat "$p"
  else
    cat "$p"
  fi
}

missing=0
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  [[ "${line#\#}" != "$line" ]] && continue
  key="${line%%=*}"
  expected="${line#*=}"
  actual="$(read_config "$config_path" | awk -F= -v k="$key" '$1==k {print $2; found=1} END {if (!found) print "<unset>"}')"
  if [[ "$actual" != "$expected" ]]; then
    echo "kernel_config_mismatch $key expected=$expected actual=$actual" >&2
    missing=1
  fi
done < "$required_fragment"

if [[ "$missing" -ne 0 ]]; then
  echo "kernel config check failed: $config_path" >&2
  exit 1
fi

echo "kernel_config_check=ok source=$config_path"
