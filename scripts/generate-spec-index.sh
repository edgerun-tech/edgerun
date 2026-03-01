#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC_DIR="${ROOT_DIR}/docs/specs"
CATALOG="${SPEC_DIR}/spec-status.tsv"
OUT="${SPEC_DIR}/ACTIVE_SPECS_INDEX.md"

if [[ ! -f "${CATALOG}" ]]; then
  echo "missing catalog: ${CATALOG}" >&2
  exit 1
fi

mapfile -t all_specs < <(find "${SPEC_DIR}" -maxdepth 1 -type f -name '*.md' -printf '%f\n' | sort)

declare -A status_by_spec=()
declare -A domain_by_spec=()
declare -A replaces_by_spec=()
declare -A note_by_spec=()

while IFS='|' read -r spec status domain replaces note; do
  [[ -z "${spec}" ]] && continue
  status_by_spec["${spec}"]="${status}"
  domain_by_spec["${spec}"]="${domain}"
  replaces_by_spec["${spec}"]="${replaces}"
  note_by_spec["${spec}"]="${note}"
done < <(awk -F '\t' 'NR > 1 { printf "%s|%s|%s|%s|%s\n", $1, $2, $3, $4, $5 }' "${CATALOG}")

active_count=0
superseded_count=0
historical_count=0
catalog_count=0
uncataloged_count=0

for spec in "${all_specs[@]}"; do
  if [[ -n "${status_by_spec[${spec}]:-}" ]]; then
    catalog_count=$((catalog_count + 1))
    case "${status_by_spec[${spec}]}" in
      active) active_count=$((active_count + 1)) ;;
      superseded) superseded_count=$((superseded_count + 1)) ;;
      historical) historical_count=$((historical_count + 1)) ;;
    esac
  else
    uncataloged_count=$((uncataloged_count + 1))
  fi
done

{
  echo "# Active Specs Index"
  echo
  echo "Generated: $(date -u +'%Y-%m-%d %H:%M:%SZ')"
  echo
  echo "## Summary"
  echo
  echo "- Total specs: ${#all_specs[@]}"
  echo "- Cataloged: ${catalog_count}"
  echo "- Active: ${active_count}"
  echo "- Superseded: ${superseded_count}"
  echo "- Historical: ${historical_count}"
  echo "- Uncataloged: ${uncataloged_count}"
  echo
  echo "## Active"
  echo
  echo "| Spec | Domain | Note |"
  echo "|---|---|---|"
  for spec in "${all_specs[@]}"; do
    [[ "${status_by_spec[${spec}]:-}" == "active" ]] || continue
    echo "| [${spec}](./${spec}) | ${domain_by_spec[${spec}]} | ${note_by_spec[${spec}]} |"
  done
  echo
  echo "## Superseded"
  echo
  echo "| Spec | Replaced By | Note |"
  echo "|---|---|---|"
  for spec in "${all_specs[@]}"; do
    [[ "${status_by_spec[${spec}]:-}" == "superseded" ]] || continue
    replacement="${replaces_by_spec[${spec}]}"
    replacement_link="${replacement}"
    if [[ -n "${replacement}" ]]; then
      replacement_link="[${replacement}](./${replacement})"
    fi
    echo "| [${spec}](./${spec}) | ${replacement_link} | ${note_by_spec[${spec}]} |"
  done
  echo
  echo "## Historical"
  echo
  echo "| Spec | Domain | Note |"
  echo "|---|---|---|"
  for spec in "${all_specs[@]}"; do
    [[ "${status_by_spec[${spec}]:-}" == "historical" ]] || continue
    echo "| [${spec}](./${spec}) | ${domain_by_spec[${spec}]} | ${note_by_spec[${spec}]} |"
  done
  echo
  echo "## Uncataloged"
  echo
  echo "These specs exist but do not yet have status metadata in \`spec-status.tsv\`."
  echo
  max_uncataloged=50
  shown_uncataloged=0
  for spec in "${all_specs[@]}"; do
    [[ -n "${status_by_spec[${spec}]:-}" ]] && continue
    if [[ ${shown_uncataloged} -ge ${max_uncataloged} ]]; then
      continue
    fi
    echo "- [${spec}](./${spec})"
    shown_uncataloged=$((shown_uncataloged + 1))
  done
  if [[ ${uncataloged_count} -gt ${max_uncataloged} ]]; then
    echo
    echo "- ... and $((uncataloged_count - max_uncataloged)) more uncataloged specs"
  fi
} > "${OUT}"

echo "generated ${OUT}"
