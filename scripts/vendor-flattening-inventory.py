#!/usr/bin/env python3
"""Inventory vendored crate trees that need flattening or removal.

Policy:
- `vendor/cargo/*` is the only temporary third-party vendor root.
- Nested `*/vendor/*` trees hide dependencies inside another crate and must be
  either deleted as dead snapshots or promoted into explicit Edgerun-owned crates.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_NAME_RE = re.compile(r'^\s*name\s*=\s*"([^"]+)"')
PACKAGE_VERSION_RE = re.compile(r'^\s*version\s*=\s*"([^"]+)"')


@dataclass(frozen=True)
class VendorManifest:
    path: Path
    name: str | None
    version: str | None
    nested: bool

    def format(self) -> str:
        rel = self.path.relative_to(ROOT)
        vendor_kind = "nested-vendor" if self.nested else "top-level-vendor"
        name = self.name or "<unknown>"
        version = self.version or "<unknown>"
        return f"{vendor_kind}: {rel}: {name} {version}"


def package_field(lines: list[str], pattern: re.Pattern[str]) -> str | None:
    in_package = False
    for line in lines:
        stripped = line.strip()
        if stripped == "[package]":
            in_package = True
            continue
        if in_package and stripped.startswith("["):
            return None
        if in_package:
            match = pattern.match(line)
            if match:
                return match.group(1)
    return None


def read_manifest(path: Path) -> VendorManifest:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    rel_parts = path.relative_to(ROOT).parts
    nested = not (len(rel_parts) >= 3 and rel_parts[0] == "vendor" and rel_parts[1] == "cargo")
    return VendorManifest(
        path=path,
        name=package_field(lines, PACKAGE_NAME_RE),
        version=package_field(lines, PACKAGE_VERSION_RE),
        nested=nested,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true", help="fail when nested vendors exist")
    args = parser.parse_args()

    manifests = sorted(
        path for path in ROOT.rglob("Cargo.toml") if "vendor" in path.relative_to(ROOT).parts
    )
    vendor_manifests = [read_manifest(path) for path in manifests]

    if not vendor_manifests:
        print("vendor manifests: none")
        return 0

    print("vendor manifests:")
    for manifest in vendor_manifests:
        print(manifest.format())

    nested_count = sum(1 for manifest in vendor_manifests if manifest.nested)
    print(f"\ntotal={len(vendor_manifests)} nested={nested_count}")
    return 1 if args.strict and nested_count else 0


if __name__ == "__main__":
    sys.exit(main())
