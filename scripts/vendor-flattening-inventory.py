#!/usr/bin/env python3
"""Inventory vendored crate trees that need flattening or removal.

Policy:
- `vendor/cargo/*` is the only temporary third-party vendor root.
- Nested `*/vendor/*` trees hide dependencies inside another crate and must be
  either deleted as dead snapshots or promoted into explicit Edgerun-owned crates.

The report also estimates whether a nested vendor tree is referenced outside of
itself. A nested tree with zero external references is a deletion candidate.
"""

from __future__ import annotations

import argparse
import re
import shlex
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_NAME_RE = re.compile(r'^\s*name\s*=\s*"([^"]+)"')
PACKAGE_VERSION_RE = re.compile(r'^\s*version\s*=\s*"([^"]+)"')
SKIP_PARTS = {".git", "target", "node_modules"}
TEXT_EXTENSIONS = {
    ".c",
    ".cc",
    ".cfg",
    ".cpp",
    ".h",
    ".hpp",
    ".json",
    ".lock",
    ".md",
    ".rs",
    ".sh",
    ".toml",
    ".txt",
    ".yaml",
    ".yml",
}


@dataclass(frozen=True)
class Reference:
    path: Path
    line: int
    needle: str

    def format(self) -> str:
        rel = self.path.relative_to(ROOT)
        return f"{rel}:{self.line}:{self.needle}"


@dataclass(frozen=True)
class VendorManifest:
    path: Path
    root: Path
    name: str | None
    version: str | None
    nested: bool
    references: tuple[Reference, ...]

    @property
    def is_delete_candidate(self) -> bool:
        return self.nested and not self.references

    def rm_command(self) -> str:
        rel = self.root.relative_to(ROOT).as_posix()
        return f"git rm -r -- {shlex.quote(rel)}"

    def format(self, *, show_references: bool) -> str:
        rel = self.path.relative_to(ROOT)
        vendor_kind = "nested-vendor" if self.nested else "top-level-vendor"
        name = self.name or "<unknown>"
        version = self.version or "<unknown>"
        reference_count = len(self.references)
        status = "delete-candidate" if self.is_delete_candidate else "referenced"
        line = f"{vendor_kind}: {rel}: {name} {version}: external_refs={reference_count}: {status}"
        if show_references and self.references:
            refs = "\n".join(f"    {reference.format()}" for reference in self.references[:20])
            if len(self.references) > 20:
                refs += f"\n    ... {len(self.references) - 20} more"
            return f"{line}\n{refs}"
        return line


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


def vendor_root_for_manifest(path: Path) -> Path:
    parent = path.parent
    if parent.parent.name == "vendor":
        return parent
    if len(parent.parts) >= 3 and parent.parts[-3:-1] == ("vendor", "cargo"):
        return parent
    return parent


def is_top_level_vendor(path: Path) -> bool:
    rel_parts = path.relative_to(ROOT).parts
    return len(rel_parts) >= 3 and rel_parts[0] == "vendor" and rel_parts[1] == "cargo"


def is_text_file(path: Path) -> bool:
    if path.name in {"Cargo.toml", "Cargo.lock", "BUILD.bazel"}:
        return True
    return path.suffix in TEXT_EXTENSIONS


def source_files() -> list[Path]:
    files: list[Path] = []
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = path.relative_to(ROOT).parts
        if any(part in SKIP_PARTS for part in rel_parts):
            continue
        if is_text_file(path):
            files.append(path)
    return files


def relative_path_needles(root: Path, name: str | None) -> list[str]:
    rel = root.relative_to(ROOT).as_posix()
    needles = {rel, f"./{rel}"}
    if name:
        needles.add(name)
        needles.add(name.replace("-", "_"))
    return sorted(needles, key=len, reverse=True)


def find_references(root: Path, name: str | None, files: list[Path]) -> tuple[Reference, ...]:
    needles = relative_path_needles(root, name)
    references: list[Reference] = []
    for path in files:
        if path == root or root in path.parents:
            continue
        try:
            lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
        except OSError:
            continue
        for line_no, line in enumerate(lines, 1):
            for needle in needles:
                if needle in line:
                    references.append(Reference(path, line_no, needle))
                    break
    return tuple(references)


def read_manifest(path: Path, files: list[Path]) -> VendorManifest:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    root = vendor_root_for_manifest(path)
    name = package_field(lines, PACKAGE_NAME_RE)
    return VendorManifest(
        path=path,
        root=root,
        name=name,
        version=package_field(lines, PACKAGE_VERSION_RE),
        nested=not is_top_level_vendor(path),
        references=find_references(root, name, files),
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true", help="fail when nested vendors exist")
    parser.add_argument(
        "--show-references",
        action="store_true",
        help="print external reference locations for each vendor tree",
    )
    parser.add_argument(
        "--print-delete-commands",
        action="store_true",
        help="print git rm commands for nested vendor trees with no external references",
    )
    args = parser.parse_args()

    manifests = sorted(
        path for path in ROOT.rglob("Cargo.toml") if "vendor" in path.relative_to(ROOT).parts
    )
    files = source_files()
    vendor_manifests = [read_manifest(path, files) for path in manifests]

    if not vendor_manifests:
        print("vendor manifests: none")
        return 0

    print("vendor manifests:")
    for manifest in vendor_manifests:
        print(manifest.format(show_references=args.show_references))

    delete_candidates = [manifest for manifest in vendor_manifests if manifest.is_delete_candidate]
    nested_count = sum(1 for manifest in vendor_manifests if manifest.nested)
    print(
        f"\ntotal={len(vendor_manifests)} nested={nested_count} "
        f"delete_candidates={len(delete_candidates)}"
    )

    if args.print_delete_commands and delete_candidates:
        print("\ndelete commands:")
        for manifest in delete_candidates:
            print(manifest.rm_command())

    return 1 if args.strict and nested_count else 0


if __name__ == "__main__":
    sys.exit(main())
