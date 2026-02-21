#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

MATRIX_FILE="docs/WHITEPAPER_IMPLEMENTATION_MATRIX.mdx"

if [ ! -f "$MATRIX_FILE" ]; then
  echo "error: missing $MATRIX_FILE" >&2
  exit 1
fi

# Validate only rows in the "Test Coverage (Implemented)" table.
# Rule: any row with Status=Implemented must have a non-empty Validation column.
awk '
BEGIN {
  in_section = 0
  in_table = 0
  bad = 0
}
/^## / {
  if ($0 ~ /^## Test Coverage \(Implemented\)/) {
    in_section = 1
  } else {
    in_section = 0
    in_table = 0
  }
  next
}
in_section && /^\|/ {
  # Start table when we hit header.
  in_table = 1
  # Skip header and separator rows.
  if ($0 ~ /^\|[[:space:]]*Scenario[[:space:]]*\|/ || $0 ~ /^\|[-[:space:]]+\|[-[:space:]]+\|[-[:space:]]+\|[[:space:]]*$/) {
    next
  }

  line = $0
  n = split(line, parts, "|")
  # parts: 1 empty, 2 Scenario, 3 Status, 4 Validation, 5 empty
  if (n < 5) {
    print "error: malformed table row: " line > "/dev/stderr"
    bad = 1
    next
  }

  status = parts[3]
  validation = parts[4]
  gsub(/^[[:space:]]+|[[:space:]]+$/, "", status)
  gsub(/^[[:space:]]+|[[:space:]]+$/, "", validation)

  if (status == "Implemented" && validation == "") {
    print "error: missing Validation for Implemented row: " line > "/dev/stderr"
    bad = 1
  }
}
END {
  if (bad) exit 1
}
' "$MATRIX_FILE"

echo "matrix validation OK: all Implemented test-coverage rows include Validation references"
