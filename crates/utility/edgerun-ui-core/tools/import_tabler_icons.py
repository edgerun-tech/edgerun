#!/usr/bin/env python3
"""Generate a tiny Rust Tabler icon table from local Tabler SVG files.

This intentionally keeps SVG/XML parsing out of edgerun-ui-core runtime.

Usage:
  npm --prefix /tmp/edgerun-tabler add @tabler/icons
  python crates/utility/edgerun-ui-core/tools/import_tabler_icons.py \
    /tmp/edgerun-tabler/node_modules/@tabler/icons/icons \
    crates/utility/edgerun-ui-core/src/tabler_generated.rs

The script searches recursively because modern Tabler packages usually store
SVG files under subfolders such as `icons/outline/` and `icons/filled/`.

The generated file stores the raw SVG path `d` attributes for selected icons.
A later renderer pass can convert these paths into line/curve primitives or an
SDF atlas. Keeping the import step separate means core stays no_std and tiny.
"""

from __future__ import annotations

import html
import re
import sys
from pathlib import Path

ICONS = [
    "brand-tabler",
    "cpu",
    "server",
    "shield-check",
    "network",
    "database",
    "terminal-2",
    "wallet",
    "key",
    "lock",
    "sparkles",
    "check",
    "alert-triangle",
]

PATH_RE = re.compile(r"<path\b[^>]*\bd=\"([^\"]+)\"", re.IGNORECASE)


def rust_string(value: str) -> str:
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"') + '"'


def find_svg(icon_dir: Path, name: str) -> Path | None:
    direct = icon_dir / f"{name}.svg"
    if direct.exists():
        return direct

    preferred = [
        icon_dir / "outline" / f"{name}.svg",
        icon_dir / "filled" / f"{name}.svg",
        icon_dir / "brands" / f"{name}.svg",
    ]
    for p in preferred:
        if p.exists():
            return p

    matches = sorted(icon_dir.rglob(f"{name}.svg"))
    if matches:
        # Prefer outline icons when present; otherwise use the first stable path.
        for p in matches:
            if "outline" in p.parts:
                return p
        return matches[0]

    return None


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    icon_dir = Path(sys.argv[1])
    out = Path(sys.argv[2])

    if not icon_dir.is_dir():
        print(f"missing icon directory: {icon_dir}", file=sys.stderr)
        return 1

    entries: list[tuple[str, list[str], Path]] = []
    missing: list[str] = []

    for name in ICONS:
        svg = find_svg(icon_dir, name)
        if svg is None:
            missing.append(name)
            continue
        text = svg.read_text(encoding="utf-8")
        paths = [html.unescape(m.group(1)).strip() for m in PATH_RE.finditer(text)]
        # Skip empty Tabler boilerplate path attrs.
        paths = [p for p in paths if p and p.lower() != "m0 0h24v24h-24z"]
        entries.append((name, paths, svg))

    if missing:
        print("missing icons: " + ", ".join(missing), file=sys.stderr)
        print("hint: inspect package layout with:", file=sys.stderr)
        print(f"  find {icon_dir} -maxdepth 3 -type f -name '*.svg' | head -40", file=sys.stderr)
        return 1

    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as f:
        f.write("//! Generated Tabler icon path table. Do not edit by hand.\n")
        f.write("//! Source: @tabler/icons SVG files.\n\n")
        f.write("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n")
        f.write("pub struct TablerIcon {\n")
        f.write("    pub name: &'static str,\n")
        f.write("    pub paths: &'static [&'static str],\n")
        f.write("}\n\n")

        for name, paths, _svg in entries:
            const_name = "PATHS_" + name.upper().replace("-", "_")
            f.write(f"const {const_name}: &[&str] = &[\n")
            for p in paths:
                f.write(f"    {rust_string(p)},\n")
            f.write("];\n\n")

        f.write("pub const TABLER_ICONS: &[TablerIcon] = &[\n")
        for name, _paths, _svg in entries:
            const_name = "PATHS_" + name.upper().replace("-", "_")
            f.write(f"    TablerIcon {{ name: {rust_string(name)}, paths: {const_name} }},\n")
        f.write("];\n")

    print(f"wrote {out} with {len(entries)} icons")
    for name, _paths, svg in entries:
        print(f"  {name}: {svg}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
