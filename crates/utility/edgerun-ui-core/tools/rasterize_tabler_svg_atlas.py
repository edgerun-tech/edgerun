#!/usr/bin/env python3
"""Rasterize selected Tabler SVGs into a high-resolution alpha atlas.

This is the correct path for crisp icons in the GPU renderer:

- SVG is rendered once at build/dev time with a real SVG renderer.
- Runtime uploads one alpha texture atlas to GL.
- Per-frame rendering is only textured quads.

Dependencies:
  Arch/CachyOS:
    sudo pacman -S python-pillow python-cairosvg

Usage:
  python crates/utility/edgerun-ui-core/tools/rasterize_tabler_svg_atlas.py \
    /tmp/edgerun-tabler/node_modules/@tabler/icons/icons \
    crates/utility/edgerun-ui-core/src/tabler_svg_atlas_generated.rs

The script searches recursively because @tabler/icons commonly stores files in
icons/outline, icons/filled, etc.
"""

from __future__ import annotations

import io
import math
import sys
from pathlib import Path

try:
    import cairosvg
    from PIL import Image
except Exception as exc:  # pragma: no cover
    raise SystemExit(
        "missing Python dependencies. Install with: sudo pacman -S python-pillow python-cairosvg\n"
        f"details: {exc}"
    )

ICONS = [
    "sparkles",
    "activity",
    "server",
    "network",
    "shield-check",
    "terminal-2",
    "wallet",
    "key",
    "lock",
    "check",
    "alert-triangle",
]

ICON_SIZE = 96
PADDING = 8
CELL = ICON_SIZE + PADDING * 2


def rust_string(value: str) -> str:
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"') + '"'


def find_svg(icon_dir: Path, name: str) -> Path | None:
    direct = icon_dir / f"{name}.svg"
    if direct.exists():
        return direct
    for sub in ("outline", "filled", "brands"):
        p = icon_dir / sub / f"{name}.svg"
        if p.exists():
            return p
    matches = sorted(icon_dir.rglob(f"{name}.svg"))
    for p in matches:
        if "outline" in p.parts:
            return p
    return matches[0] if matches else None


def rasterize_alpha(svg_path: Path) -> Image.Image:
    svg = svg_path.read_text(encoding="utf-8")
    # Force currentColor to white so alpha can be extracted reliably.
    svg = svg.replace('currentColor', '#ffffff')
    png = cairosvg.svg2png(bytestring=svg.encode("utf-8"), output_width=ICON_SIZE, output_height=ICON_SIZE)
    rgba = Image.open(io.BytesIO(png)).convert("RGBA")
    return rgba.getchannel("A")


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    icon_dir = Path(sys.argv[1])
    out = Path(sys.argv[2])
    if not icon_dir.is_dir():
        print(f"missing icon directory: {icon_dir}", file=sys.stderr)
        return 1

    found: list[tuple[str, Path]] = []
    missing: list[str] = []
    for name in ICONS:
        svg = find_svg(icon_dir, name)
        if svg is None:
            missing.append(name)
        else:
            found.append((name, svg))

    if missing:
        print("missing icons: " + ", ".join(missing), file=sys.stderr)
        return 1

    cols = math.ceil(math.sqrt(len(found)))
    rows = math.ceil(len(found) / cols)
    atlas_w = cols * CELL
    atlas_h = rows * CELL
    atlas = Image.new("L", (atlas_w, atlas_h), 0)

    entries = []
    for idx, (name, svg) in enumerate(found):
        col = idx % cols
        row = idx // cols
        x = col * CELL + PADDING
        y = row * CELL + PADDING
        alpha = rasterize_alpha(svg)
        atlas.paste(alpha, (x, y))
        entries.append((name, x, y, ICON_SIZE, ICON_SIZE, svg))

    data = list(atlas.tobytes())
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as f:
        f.write("//! Generated Tabler SVG alpha atlas. Do not edit by hand.\n")
        f.write("//! Source: @tabler/icons rendered by tools/rasterize_tabler_svg_atlas.py.\n\n")
        f.write(f"pub const TABLER_SVG_ATLAS_W: u32 = {atlas_w};\n")
        f.write(f"pub const TABLER_SVG_ATLAS_H: u32 = {atlas_h};\n")
        f.write(f"pub const TABLER_SVG_ICON_SIZE: u32 = {ICON_SIZE};\n\n")
        f.write("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n")
        f.write("pub struct SvgIconRect {\n")
        f.write("    pub name: &'static str,\n")
        f.write("    pub x: u32,\n")
        f.write("    pub y: u32,\n")
        f.write("    pub w: u32,\n")
        f.write("    pub h: u32,\n")
        f.write("}\n\n")
        f.write("pub const TABLER_SVG_ICONS: &[SvgIconRect] = &[\n")
        for name, x, y, w, h, _svg in entries:
            f.write(f"    SvgIconRect {{ name: {rust_string(name)}, x: {x}, y: {y}, w: {w}, h: {h} }},\n")
        f.write("];\n\n")
        f.write("pub const TABLER_SVG_ATLAS_ALPHA: &[u8] = &[\n")
        for i in range(0, len(data), 24):
            chunk = ", ".join(str(v) for v in data[i:i+24])
            f.write(f"    {chunk},\n")
        f.write("];\n")

    print(f"wrote {out}")
    print(f"atlas: {atlas_w}x{atlas_h}, icons: {len(entries)}")
    for name, _x, _y, _w, _h, svg in entries:
        print(f"  {name}: {svg}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
