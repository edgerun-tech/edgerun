#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "[workflow-refs] checking workflow_run references"

declare -A workflow_names=()
while IFS= read -r wf; do
  name_line="$(sed -n '1,20p' "$wf" | rg -n '^name:' -N || true)"
  if [[ -n "${name_line}" ]]; then
    wf_name="${name_line#name: }"
    wf_name="${wf_name%$'\r'}"
    workflow_names["${wf_name}"]="$wf"
  fi
done < <(find .github/workflows -maxdepth 1 -name '*.yml' | sort)

failures=0

extract_refs() {
  local file="$1"
  awk '
    /^[[:space:]]*workflow_run:/ {in_wr=1; in_list=0; next}
    in_wr && /^[[:space:]]*workflow_dispatch:/ {in_wr=0; in_list=0}
    in_wr && /^[[:space:]]*workflows:[[:space:]]*\[/ {
      line=$0
      sub(/.*workflows:[[:space:]]*\[/, "", line)
      sub(/\].*/, "", line)
      gsub(/"/, "", line)
      n=split(line, arr, /,/) 
      for (i=1; i<=n; i++) {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", arr[i])
        if (arr[i] != "") print arr[i]
      }
      next
    }
    in_wr && /^[[:space:]]*workflows:[[:space:]]*$/ {in_list=1; next}
    in_wr && in_list && /^[[:space:]]*-[[:space:]]*/ {
      line=$0
      sub(/^[[:space:]]*-[[:space:]]*/, "", line)
      gsub(/"/, "", line)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
      if (line != "") print line
      next
    }
    in_wr && in_list && /^[[:space:]]*[a-z_]+:/ {in_list=0}
  ' "$file"
}

while IFS= read -r wf; do
  while IFS= read -r ref_name; do
    [[ -z "$ref_name" ]] && continue
    if [[ -z "${workflow_names[$ref_name]+x}" ]]; then
      echo "[workflow-refs] FAIL ${wf}: references missing workflow name '${ref_name}'"
      failures=$((failures + 1))
    else
      echo "[workflow-refs] PASS ${wf}: '${ref_name}' -> ${workflow_names[$ref_name]}"
    fi
  done < <(extract_refs "$wf")
done < <(find .github/workflows -maxdepth 1 -name '*.yml' | sort)

if [[ "$failures" -gt 0 ]]; then
  echo "[workflow-refs] detected ${failures} missing workflow reference(s)"
  exit 1
fi

echo "[workflow-refs] all workflow_run references resolved"
