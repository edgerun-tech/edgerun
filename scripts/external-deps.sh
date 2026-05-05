#!/usr/bin/env bash
set -euo pipefail

metadata="${EXTERNAL_DEPS_METADATA:-}"
if [[ -z "$metadata" ]]; then
    metadata="$(mktemp "${TMPDIR:-/tmp}/edgerun-metadata.XXXXXX.json")"
    trap 'rm -f "$metadata"' EXIT
    cargo metadata --format-version 1 >"$metadata"
fi

python3 - "$metadata" <<'PY'
import json
import sys
from collections import defaultdict

metadata_path = sys.argv[1]
with open(metadata_path, "r", encoding="utf-8") as fh:
    metadata = json.load(fh)

packages = {package["id"]: package for package in metadata["packages"]}
resolve = metadata.get("resolve") or {"nodes": []}
nodes = {node["id"]: node for node in resolve["nodes"]}
workspace = set(metadata["workspace_members"])
external = {
    package_id
    for package_id, package in packages.items()
    if package.get("source") is not None
}
resolved_external = sorted(
    (packages[package_id]["name"], packages[package_id]["version"])
    for package_id in nodes
    if package_id in external
)

direct = defaultdict(list)
for package_id in sorted(workspace, key=lambda value: packages[value]["name"]):
    node = nodes.get(package_id)
    if not node:
        continue
    for dep in node.get("deps", []):
        dep_id = dep["pkg"]
        if dep_id in external:
            package = packages[dep_id]
            direct[packages[package_id]["name"]].append(
                (dep["name"], package["name"], package["version"])
            )

print(f"workspace_members={len(workspace)}")
print(f"resolved_external_packages={len(resolved_external)}")
print()
print("direct_external_dependencies:")
if direct:
    for package_name in sorted(direct):
        deps = ", ".join(
            f"{name} {version}" if alias == name else f"{alias}->{name} {version}"
            for alias, name, version in sorted(direct[package_name])
        )
        print(f"  {package_name}: {deps}")
else:
    print("  none")

print()
print("resolved_external_packages:")
for name, version in resolved_external:
    print(f"  {name} {version}")
PY
