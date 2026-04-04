#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
buf generate
find go/gen/lifegraph/v0 -name '*.pb.go' -print0 | while IFS= read -r -d '' path; do
  perl -0pi -e 's/,json=[^,`]+//g; s/ json:"[^"]*"//g' "$path"
done
