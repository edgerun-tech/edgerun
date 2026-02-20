#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <profile_a.json> <profile_b.json>"
  exit 1
fi

python3 - "$1" "$2" <<'PY'
import json
import sys

a_path, b_path = sys.argv[1], sys.argv[2]
with open(a_path, "r", encoding="utf-8") as f:
    a = json.load(f)
with open(b_path, "r", encoding="utf-8") as f:
    b = json.load(f)

def normalize(doc):
    out = {}
    for case in doc.get("cases", []):
        out[case["case"]] = {
            "actual": case.get("actual"),
            "passed": case.get("passed"),
            "stable": case.get("stable"),
        }
    return out

a_cases = normalize(a)
b_cases = normalize(b)

if a_cases != b_cases:
    print("replay profile mismatch detected")
    print("left:", a_path)
    print("right:", b_path)
    print("left_cases=", a_cases)
    print("right_cases=", b_cases)
    sys.exit(2)

if not all(v["passed"] and v["stable"] for v in a_cases.values()):
    print("replay cases are not fully passed/stable in first profile")
    sys.exit(3)

print("replay profiles match and all cases passed/stable")
PY
