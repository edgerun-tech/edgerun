#!/usr/bin/env python3
"""Report external dependency escape hatches in the Edgerun workspace.

The long-term policy is that reusable Edgerun code should depend on Edgerun-owned
crates only. Vendored crates and crates.io dependencies are migration shims, not
the target architecture.

This script intentionally avoids `cargo metadata` so it can catch manifest-level
issues even when the workspace is temporarily not compiling.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# These names are Rust language/toolchain crates or intentionally external until
# an Edgerun-owned no_std replacement is implemented. Keep this list short and
# treat every addition as technical debt.
ALLOW_EXTERNAL_NAMES = {
    "alloc",
    "core",
    "libc",
    "proc-macro2",
    "quote",
    "rkyv",
    "syn",
}

# Directories that are explicitly not first-class Edgerun source.
SKIP_PARTS = {
    ".git",
    "target",
    "node_modules",
}

DEPENDENCY_TABLE_RE = re.compile(r"^\s*\[(?:target\.[^]]+\.)?(?:build-)?(?:dev-)?dependencies(?:\.[^]]+)?]\s*$")
PACKAGE_TABLE_RE = re.compile(r"^\s*\[package]\s*$")
TABLE_RE = re.compile(r"^\s*\[[^]]+]\s*$")
DEPENDENCY_RE = re.compile(r"^\s*([A-Za-z0-9_.-]+)\s*=\s*(.+?)\s*(?:#.*)?$")
PACKAGE_NAME_RE = re.compile(r'^\s*name\s*=\s*"([^"]+)"')
INLINE_PACKAGE_RE = re.compile(r'\bpackage\s*=\s*"([^"]+)"')
INLINE_PATH_RE = re.compile(r'\bpath\s*=\s*"([^"]+)"')
INLINE_VERSION_RE = re.compile(r'\bversion\s*=\s*"([^"]+)"')


@dataclass(frozen=True)
class Finding:
    kind: str
    path: Path
    line: int
    message: str

    def format(self) -> str:
        rel = self.path.relative_to(ROOT)
        return f"{self.kind}: {rel}:{self.line}: {self.message}"


def cargo_manifests() -> list[Path]:
    manifests: list[Path] = []
    for path in ROOT.rglob("Cargo.toml"):
        if any(part in SKIP_PARTS for part in path.parts):
            continue
        manifests.append(path)
    return sorted(manifests)


def package_name(manifest: Path, lines: list[str]) -> str | None:
    in_package = False
    for line in lines:
        if PACKAGE_TABLE_RE.match(line):
            in_package = True
            continue
        if in_package and TABLE_RE.match(line):
            return None
        if in_package:
            match = PACKAGE_NAME_RE.match(line)
            if match:
                return match.group(1)
    return None


def dep_package_name(dep_key: str, dep_value: str) -> str:
    match = INLINE_PACKAGE_RE.search(dep_value)
    if match:
        return match.group(1)
    return dep_key


def has_path(dep_value: str) -> bool:
    return INLINE_PATH_RE.search(dep_value) is not None


def has_workspace(dep_value: str) -> bool:
    return "workspace" in dep_value


def has_version(dep_value: str) -> bool:
    return INLINE_VERSION_RE.search(dep_value) is not None


def is_external_name(name: str) -> bool:
    return not name.startswith("edgerun-") and not name.startswith("codex-")


def scan_manifest(manifest: Path) -> list[Finding]:
    text = manifest.read_text(encoding="utf-8")
    lines = text.splitlines()
    own_package = package_name(manifest, lines)
    findings: list[Finding] = []
    in_dependencies = False

    for idx, line in enumerate(lines, 1):
        if DEPENDENCY_TABLE_RE.match(line):
            in_dependencies = True
            continue
        if TABLE_RE.match(line):
            in_dependencies = False
            continue
        if not in_dependencies:
            continue

        match = DEPENDENCY_RE.match(line)
        if not match:
            continue

        dep_key, dep_value = match.groups()
        package = dep_package_name(dep_key, dep_value)

        if "vendor/cargo" in dep_value:
            findings.append(
                Finding(
                    "vendor-dependency",
                    manifest,
                    idx,
                    f"{dep_key} points at vendor/cargo; replace with an edgerun-owned implementation",
                )
            )

        if package in ALLOW_EXTERNAL_NAMES:
            continue

        if has_path(dep_value) or has_workspace(dep_value):
            if is_external_name(package) and own_package != package:
                findings.append(
                    Finding(
                        "external-path-dependency",
                        manifest,
                        idx,
                        f"{dep_key} resolves to non-edgerun package {package}; create/use an edgerun-* crate",
                    )
                )
            continue

        # A plain version dependency means crates.io or another registry.
        if has_version(dep_value) or dep_value.lstrip().startswith('"'):
            findings.append(
                Finding(
                    "registry-dependency",
                    manifest,
                    idx,
                    f"{dep_key} is registry-backed; create/use an edgerun-* crate",
                )
            )

    return findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--strict",
        action="store_true",
        help="exit non-zero when any finding is present",
    )
    args = parser.parse_args()

    findings: list[Finding] = []
    for manifest in cargo_manifests():
        findings.extend(scan_manifest(manifest))

    if findings:
        print("Dependency sovereignty findings:")
        for finding in findings:
            print(finding.format())
        print(f"\nfindings={len(findings)}")
        return 1 if args.strict else 0

    print("Dependency sovereignty findings: none")
    return 0


if __name__ == "__main__":
    sys.exit(main())
