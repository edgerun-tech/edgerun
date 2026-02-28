#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

# Normalizes SPDX headers across tracked source files in this repository.
# Usage:
#   scripts/spdx-normalize.sh

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

license_for_path() {
  local path="$1"

  case "$path" in
    crates/*|docs/*|scripts/*|frontend/*|edgerun-apps/*) echo "Apache-2.0" ;;
    *)
      echo ""
      ;;
  esac
}

comment_prefix_for_path() {
  local path="$1"
  case "$path" in
    *.rs|*.ts|*.tsx|*.js|*.jsx|*.mjs|*.cjs) echo "//" ;;
    *.sh|*.bash|*.zsh|*.py|*.yml|*.yaml|*.toml) echo "#" ;;
    *) echo "" ;;
  esac
}

normalize_file() {
  local path="$1"
  local license_id="$2"
  local prefix="$3"
  local header="${prefix} SPDX-License-Identifier: ${license_id}"
  local tmp
  tmp="$(mktemp)"

  if [[ "$prefix" == "#" ]] && head -n 1 "$path" | grep -q '^#!'; then
    head -n 1 "$path" >"$tmp"
    tail -n +2 "$path" >"${tmp}.body"
    if grep -q "SPDX-License-Identifier:" "${tmp}.body"; then
      awk -v header="$header" '
        BEGIN { done=0 }
        {
          if (!done && $0 ~ /SPDX-License-Identifier:/) {
            print header
            done=1
            next
          }
          print
        }
      ' "${tmp}.body" >>"$tmp"
    else
      {
        echo "$header"
        cat "${tmp}.body"
      } >>"$tmp"
    fi
    rm -f "${tmp}.body"
  else
    if grep -q "SPDX-License-Identifier:" "$path"; then
      awk -v header="$header" '
        BEGIN { done=0 }
        {
          if (!done && $0 ~ /SPDX-License-Identifier:/) {
            print header
            done=1
            next
          }
          print
        }
      ' "$path" >"$tmp"
    else
      {
        echo "$header"
        cat "$path"
      } >"$tmp"
    fi
  fi

  if ! cmp -s "$path" "$tmp"; then
    mv "$tmp" "$path"
    echo "updated: $path"
  else
    rm -f "$tmp"
  fi
}

while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  [[ "$path" == *"/LICENSE" || "$path" == "LICENSE" ]] && continue
  case "$path" in
    out/*)
      continue
      ;;
  esac

  license_id="$(license_for_path "$path")"
  [[ -z "$license_id" ]] && continue

  prefix="$(comment_prefix_for_path "$path")"
  [[ -z "$prefix" ]] && continue

  normalize_file "$path" "$license_id" "$prefix"
done < <(git ls-files)

echo "SPDX normalization complete."
