#!/usr/bin/env python3
"""Report canonical UiIcon coverage across generated SVG atlases.

Usage:
  python crates/utility/edgerun-ui-core/tools/report_icon_coverage.py
  python crates/utility/edgerun-ui-core/tools/report_icon_coverage.py --format markdown
  python crates/utility/edgerun-ui-core/tools/report_icon_coverage.py --write /tmp/icon-coverage.md
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[4]
CRATE_ROOT = REPO_ROOT / "crates" / "utility" / "edgerun-ui-core"
ICONS_RS = CRATE_ROOT / "src" / "gpu" / "icons.rs"
ATLAS_FILES = {
    "tabler": CRATE_ROOT / "src" / "tabler_svg_atlas_generated.rs",
    "lucide": CRATE_ROOT / "src" / "lucide_svg_atlas_generated.rs",
}


@dataclass(frozen=True)
class ProviderCoverage:
    provider: str
    mapped_icons: int
    canonical_icons: int
    unique_provider_names: int
    atlas_entries: int
    missing: tuple[str, ...]
    unused: tuple[str, ...]
    aliases: tuple[str, ...]

    @property
    def complete(self) -> bool:
        return not self.missing


def pascal_to_kebab(value: str) -> str:
    out = []
    for idx, ch in enumerate(value):
        if ch.isupper() and idx > 0:
            out.append("-")
        out.append(ch.lower())
    return "".join(out)


def extract_block(text: str, marker: str) -> str:
    match = re.search(rf"{re.escape(marker)}\b", text)
    if match is None:
        raise ValueError(f"missing marker: {marker}")
    brace = text.find("{", match.start())
    if brace < 0:
        raise ValueError(f"missing opening brace after: {marker}")

    depth = 0
    for idx in range(brace, len(text)):
        if text[idx] == "{":
            depth += 1
        elif text[idx] == "}":
            depth -= 1
            if depth == 0:
                return text[brace + 1 : idx]
    raise ValueError(f"unterminated block: {marker}")


def canonical_icons(text: str) -> list[str]:
    block = extract_block(text, "pub enum UiIcon")
    icons: list[str] = []
    for line in block.splitlines():
        line = line.strip().rstrip(",")
        if line and re.fullmatch(r"[A-Z][A-Za-z0-9_]*", line):
            icons.append(line)
    if not icons:
        raise ValueError("no canonical UiIcon variants found")
    return icons


def provider_mapping(text: str, function_name: str) -> dict[str, str]:
    block = extract_block(text, f"pub const fn {function_name}")
    mapping: dict[str, str] = {}
    for variants, provider_name in re.findall(
        r"((?:Self::[A-Za-z0-9_]+\s*(?:\|\s*)?)+)\s*=>\s*\"([^\"]+)\"",
        block,
    ):
        for variant in re.findall(r"Self::([A-Za-z0-9_]+)", variants):
            mapping[variant] = provider_name
    return mapping


def atlas_names(path: Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    return set(re.findall(r'name:\s*"([^"]+)"', text))


def provider_coverage(
    provider: str,
    icons: list[str],
    mapping: dict[str, str],
    atlas: set[str],
) -> ProviderCoverage:
    missing_mapping = [icon for icon in icons if icon not in mapping]
    missing_atlas = [
        f"{icon}->{mapping[icon]}" for icon in icons if icon in mapping and mapping[icon] not in atlas
    ]
    missing = tuple(missing_mapping + missing_atlas)
    used_names = [mapping[icon] for icon in icons if icon in mapping]
    unique_used = set(used_names)
    aliases = tuple(
        sorted(
            name
            for name in unique_used
            if sum(1 for used in used_names if used == name) > 1
        )
    )
    unused = tuple(sorted(atlas - unique_used))
    return ProviderCoverage(
        provider=provider,
        mapped_icons=len(used_names),
        canonical_icons=len(icons),
        unique_provider_names=len(unique_used),
        atlas_entries=len(atlas),
        missing=missing,
        unused=unused,
        aliases=aliases,
    )


def build_report() -> tuple[list[str], list[ProviderCoverage]]:
    icon_text = ICONS_RS.read_text(encoding="utf-8")
    icons = canonical_icons(icon_text)
    providers = [
        provider_coverage(
            "tabler",
            icons,
            provider_mapping(icon_text, "tabler_name"),
            atlas_names(ATLAS_FILES["tabler"]),
        ),
        provider_coverage(
            "lucide",
            icons,
            provider_mapping(icon_text, "lucide_name"),
            atlas_names(ATLAS_FILES["lucide"]),
        ),
    ]
    return icons, providers


def render_text(icons: list[str], providers: list[ProviderCoverage]) -> str:
    lines = [f"Icon coverage: {len(icons)} canonical UiIcon variants"]
    for coverage in providers:
        state = "complete" if coverage.complete else "missing"
        lines.append(
            f"- {coverage.provider}: {state}; "
            f"{coverage.mapped_icons}/{coverage.canonical_icons} mapped, "
            f"{coverage.unique_provider_names}/{coverage.atlas_entries} atlas names used"
        )
        if coverage.aliases:
            lines.append(f"  aliases: {', '.join(coverage.aliases)}")
        if coverage.unused:
            lines.append(f"  unused atlas entries: {', '.join(coverage.unused)}")
        if coverage.missing:
            lines.append(f"  missing: {', '.join(coverage.missing)}")
    return "\n".join(lines) + "\n"


def render_markdown(icons: list[str], providers: list[ProviderCoverage]) -> str:
    lines = [
        "# Icon Coverage",
        "",
        f"Canonical `UiIcon` variants: `{len(icons)}`",
        "",
        "| Provider | Status | Mapped | Atlas Names Used | Aliased Names | Unused Atlas Entries |",
        "| --- | --- | ---: | ---: | --- | --- |",
    ]
    for coverage in providers:
        status = "complete" if coverage.complete else "missing"
        aliases = ", ".join(f"`{name}`" for name in coverage.aliases) or "-"
        unused = ", ".join(f"`{name}`" for name in coverage.unused) or "-"
        lines.append(
            f"| {coverage.provider} | {status} | "
            f"{coverage.mapped_icons}/{coverage.canonical_icons} | "
            f"{coverage.unique_provider_names}/{coverage.atlas_entries} | "
            f"{aliases} | {unused} |"
        )
    missing = [
        f"- `{coverage.provider}`: {', '.join(coverage.missing)}"
        for coverage in providers
        if coverage.missing
    ]
    if missing:
        lines.extend(["", "## Missing", "", *missing])
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--format", choices=("text", "markdown"), default="text")
    parser.add_argument("--write", type=Path)
    args = parser.parse_args()

    icons, providers = build_report()
    report = render_markdown(icons, providers) if args.format == "markdown" else render_text(icons, providers)
    if args.write:
        args.write.parent.mkdir(parents=True, exist_ok=True)
        args.write.write_text(report, encoding="utf-8")
    else:
        print(report, end="")

    return 0 if all(provider.complete for provider in providers) else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"icon coverage report failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
