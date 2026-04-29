#!/usr/bin/env python3
"""Inspect the ESP32-S3 unikernel ELF memory layout.

This tool intentionally depends only on the local Xtensa binutils that the
ESP32-S3 build already uses. It is for bring-up: make linker/layout surprises
obvious before flashing a blob-linked image.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


REGIONS = {
    "drom": (0x3C000020, 0x3C000020 + 4 * 1024 * 1024, "R"),
    "irom": (0x42000020, 0x42000020 + 4 * 1024 * 1024, "RX"),
    "iram": (0x40380000, 0x403B9000, "RX"),
    "dram": (0x3FC88000, 0x3FCDB700, "RW"),
}

BLOB_SECTION_PREFIXES = (
    ".dram1",
    ".iram1",
    ".wifi",
    ".wifirx",
    ".wifislp",
    ".wifiextra",
    ".rodata_wlog",
)

KEY_SYMBOLS = (
    "_start",
    "kernel_main",
    "_bss_start",
    "_bss_end",
    "_stack_end",
    "_stack",
    "_end",
    "g_cnxMgr",
    "g_misc_nvs",
    "g_osi_funcs_p",
    "WIFI_EVENT",
)


@dataclass(frozen=True)
class Section:
    index: int
    name: str
    type: str
    addr: int
    offset: int
    size: int
    flags: str
    align: int

    @property
    def end(self) -> int:
        return self.addr + self.size

    @property
    def alloc(self) -> bool:
        return "A" in self.flags

    @property
    def writable(self) -> bool:
        return "W" in self.flags

    @property
    def executable(self) -> bool:
        return "X" in self.flags

    @property
    def blob_related(self) -> bool:
        return self.name.startswith(BLOB_SECTION_PREFIXES)


@dataclass(frozen=True)
class Symbol:
    name: str
    addr: int
    size: int | None
    type: str

    @property
    def end(self) -> int:
        return self.addr + (self.size or 0)


SECTION_RE = re.compile(
    r"^\s*\[\s*(?P<idx>\d+)\]\s+"
    r"(?P<name>\S+)\s+"
    r"(?P<type>\S+)\s+"
    r"(?P<addr>[0-9a-fA-F]+)\s+"
    r"(?P<off>[0-9a-fA-F]+)\s+"
    r"(?P<size>[0-9a-fA-F]+)\s+"
    r"(?P<entsize>[0-9a-fA-F]+)\s+"
    r"(?P<flags>\S*)\s+"
    r"(?P<link>\d+)\s+"
    r"(?P<info>\d+)\s+"
    r"(?P<align>\d+)"
)


def run_tool(toolchain_bin: Path, tool: str, args: list[str]) -> str:
    exe = toolchain_bin / f"xtensa-esp32s3-elf-{tool}"
    if not exe.exists():
        exe = toolchain_bin / f"xtensa-esp-elf-{tool}"
    if not exe.exists():
        raise SystemExit(f"missing Xtensa tool: {tool} in {toolchain_bin}")
    return subprocess.check_output([str(exe), *args], text=True, stderr=subprocess.STDOUT)


def parse_sections(readelf_output: str) -> list[Section]:
    sections: list[Section] = []
    for line in readelf_output.splitlines():
        match = SECTION_RE.match(line)
        if not match:
            continue
        sections.append(
            Section(
                index=int(match.group("idx")),
                name=match.group("name"),
                type=match.group("type"),
                addr=int(match.group("addr"), 16),
                offset=int(match.group("off"), 16),
                size=int(match.group("size"), 16),
                flags=match.group("flags"),
                align=int(match.group("align")),
            )
        )
    return sections


def parse_symbols(nm_output: str) -> dict[str, Symbol]:
    symbols: dict[str, Symbol] = {}
    for line in nm_output.splitlines():
        parts = line.split()
        if len(parts) == 4:
            addr_s, size_s, type_s, name = parts
            size = int(size_s, 16)
        elif len(parts) == 3:
            addr_s, type_s, name = parts
            size = None
        else:
            continue
        if not re.fullmatch(r"[0-9a-fA-F]+", addr_s):
            continue
        symbols[name] = Symbol(name=name, addr=int(addr_s, 16), size=size, type=type_s)
    return symbols


def region_for(addr: int, size: int = 1) -> str | None:
    if size == 0:
        size = 1
    end = addr + size
    for name, (start, limit, _perms) in REGIONS.items():
        if start <= addr and end <= limit:
            return name
    return None


def fmt_range(start: int, end: int) -> str:
    return f"0x{start:08x}..0x{end:08x}"


def print_region_summary(sections: list[Section]) -> None:
    print("ESP32-S3 memory regions")
    for name, (start, limit, perms) in REGIONS.items():
        region_sections = [s for s in sections if s.alloc and region_for(s.addr, s.size) == name]
        used = 0
        if region_sections:
            used = max(s.end for s in region_sections) - start
        print(
            f"  {name:4} {perms:2} {fmt_range(start, limit)} "
            f"used={used:6} / {limit - start}"
        )


def print_sections(title: str, sections: list[Section], limit: int | None = None) -> None:
    print(title)
    shown = sections if limit is None else sections[:limit]
    if not shown:
        print("  none")
        return
    for section in shown:
        region = region_for(section.addr, section.size) or "?"
        print(
            f"  {section.name:24} {region:4} {fmt_range(section.addr, section.end)} "
            f"size={section.size:6} flags={section.flags or '-'}"
        )
    if limit is not None and len(sections) > limit:
        print(f"  ... {len(sections) - limit} more")


def print_symbols(symbols: dict[str, Symbol]) -> None:
    print("Key symbols")
    for name in KEY_SYMBOLS:
        sym = symbols.get(name)
        if sym is None:
            print(f"  {name:16} missing")
            continue
        region = region_for(sym.addr, sym.size or 1) or "?"
        size = "-" if sym.size is None else str(sym.size)
        print(f"  {name:16} {region:4} 0x{sym.addr:08x} size={size} type={sym.type}")


def print_bss_symbols(symbols: dict[str, Symbol]) -> None:
    bss_start = symbols.get("_bss_start")
    bss_end = symbols.get("_bss_end")
    if bss_start is None or bss_end is None or bss_end.addr <= bss_start.addr:
        return
    contained = [
        sym
        for sym in symbols.values()
        if sym.size
        and sym.type in ("B", "b", "C")
        and bss_start.addr <= sym.addr
        and sym.end <= bss_end.addr
    ]
    contained.sort(key=lambda sym: sym.size or 0, reverse=True)
    print("Largest boot-cleared BSS symbols")
    if not contained:
        print("  none")
        return
    for sym in contained[:20]:
        print(f"  {sym.name:32} 0x{sym.addr:08x} size={sym.size}")


def collect_errors(sections: list[Section], symbols: dict[str, Symbol]) -> list[str]:
    errors: list[str] = []
    for section in sections:
        if not section.alloc or section.size == 0:
            continue
        region = region_for(section.addr, section.size)
        if region is None:
            errors.append(
                f"section {section.name} outside known regions at {fmt_range(section.addr, section.end)}"
            )
            continue
        if section.executable and region not in ("iram", "irom"):
            errors.append(f"executable section {section.name} is in {region}")
        if section.writable and region not in ("dram",):
            errors.append(f"writable section {section.name} is in {region}")

    bss_start = symbols.get("_bss_start")
    bss_end = symbols.get("_bss_end")
    stack = symbols.get("_stack")
    stack_end = symbols.get("_stack_end")
    if bss_start and bss_end and bss_end.addr < bss_start.addr:
        errors.append("_bss_end is before _bss_start")
    if stack and stack_end and stack.addr < stack_end.addr:
        errors.append("_stack is before _stack_end")
    if bss_end and stack_end and stack_end.addr < bss_end.addr:
        errors.append("_stack_end is before _bss_end")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("elf", type=Path)
    parser.add_argument(
        "--toolchain-bin",
        type=Path,
        default=Path(
            os.environ.get(
                "ESP_TOOLCHAIN_BIN",
                "devices/edgerun-tcl-usb-ap-bridge/.embuild/espressif/tools/"
                "xtensa-esp-elf/esp-15.2.0_20251204/xtensa-esp-elf/bin",
            )
        ),
    )
    parser.add_argument("--strict", action="store_true", help="fail on warnings too")
    args = parser.parse_args()

    elf = args.elf.resolve()
    toolchain_bin = args.toolchain_bin.resolve()
    if not elf.exists():
        raise SystemExit(f"missing ELF: {elf}")

    readelf = run_tool(toolchain_bin, "readelf", ["-SW", str(elf)])
    nm = run_tool(toolchain_bin, "nm", ["-S", "-n", str(elf)])
    sections = parse_sections(readelf)
    symbols = parse_symbols(nm)

    alloc_sections = [s for s in sections if s.alloc and s.size != 0]
    blob_sections = [s for s in alloc_sections if s.blob_related]
    unknown_sections = [s for s in alloc_sections if region_for(s.addr, s.size) is None]

    print_region_summary(sections)
    print()
    print_symbols(symbols)
    print()
    print_bss_symbols(symbols)
    print()
    print_sections("Blob-related alloc sections", blob_sections, limit=80)
    print()
    print_sections("Alloc sections outside known regions", unknown_sections)

    errors = collect_errors(sections, symbols)
    if errors:
        print()
        print("Layout errors")
        for error in errors:
            print(f"  error: {error}")
        return 1

    if args.strict and not blob_sections:
        print()
        print("Layout errors")
        print("  error: --strict expected blob-related sections, found none")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
