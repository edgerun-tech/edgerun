#!/usr/bin/env python3
"""Scan build/wasm/ for .wat files and emit module-registry.json + module-registry.wat.

Usage: python3 build-registry.py [--json PATH] [--wat PATH]
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / "build" / "wasm"
REGISTRY_DIR = Path(__file__).resolve().parent
DEFAULT_JSON = REGISTRY_DIR / "module-registry.json"
DEFAULT_WAT = REGISTRY_DIR / "module-registry.wat"

# Compile a .wat to .wasm and extract exports via wasm-objdump
def extract_exports(wat_path):
    try:
        wasm = subprocess.run(
            ["wat2wasm", str(wat_path), "-o", "-"],
            capture_output=True, timeout=30
        )
        if wasm.returncode != 0:
            return None
        dump = subprocess.run(
            ["wasm-objdump", "-x", "-"],
            input=wasm.stdout, capture_output=True, timeout=10
        )
        if dump.returncode != 0:
            return None
        exports = []
        in_export = False
        for line in dump.stdout.decode().split("\n"):
            if "Export[" in line:
                in_export = True
                continue
            if in_export:
                m = re.match(r'\s*-\s*(\w+)\[(\d+)\]\s+[<.]([^>]+)[>.].*->\s*"([^"]+)"', line)
                if m:
                    exports.append({
                        "kind": m.group(1),
                        "idx": int(m.group(2)),
                        "internal": m.group(3),
                        "export": m.group(4)
                    })
                elif line.strip() and not line.startswith(" " * 2):
                    break
                elif "Code[" in line:
                    break
        return exports
    except:
        return None


def extract_standard_id(wat_path):
    text = wat_path.read_text(encoding="utf-8", errors="replace")
    # Pattern: "proto_standard_id") (result i32) (i32.const 300xxx)
    m = re.search(r'proto_standard_id["\']?\)[^)]*\(result\s+i32\)[^)]*i32\.const\s+(\d+)', text)
    if m:
        return int(m.group(1))
    # Pattern using global: proto_standard_id) (result i32) (global.get $STANDARD_ID)
    m = re.search(r'proto_standard_id["\']?\)[^)]*\(result\s+i32\)[^)]*global\.get\s+\$(\w+)', text)
    if m:
        # Try to find the global definition
        gname = m.group(1)
        gm = re.search(rf'\(global\s+\${gname}\s+i32\s*\(i32\.const\s+(\d+)\)\)', text)
        if gm:
            return int(gm.group(1))
    return None


def extract_abi_version(wat_path):
    text = wat_path.read_text(encoding="utf-8", errors="replace")
    m = re.search(r'proto_abi_version["\']?\)[^)]*\(result\s+i32\)[^)]*i32\.const\s+(\d+)', text)
    if m:
        return int(m.group(1))
    m = re.search(r'proto_abi_version["\']?\)[^)]*\(result\s+i32\)[^)]*global\.get\s+\$(\w+)', text)
    if m:
        gname = m.group(1)
        gm = re.search(rf'\(global\s+\${gname}\s+i32\s*\(i32\.const\s+(\d+)\)\)', text)
        if gm:
            return int(gm.group(1))
    return None


def has_simd_capabilities(wat_path):
    text = wat_path.read_text(encoding="utf-8", errors="replace")
    return '"simd_capabilities"' in text or 'simd_capabilities' in text


def has_memory_export(wat_path):
    text = wat_path.read_text(encoding="utf-8", errors="replace")
    return '(export "memory")' in text


def get_line_count(wat_path):
    text = wat_path.read_text(encoding="utf-8", errors="replace")
    return text.count("\n") + 1


def scan_directory(category_dir):
    modules = {}
    for fpath in sorted(category_dir.rglob("*.wat")):
        rel = fpath.relative_to(BUILD)
        mod_name = str(rel.with_suffix(""))
        sid = extract_standard_id(fpath)
        abi = extract_abi_version(fpath)
        info = {
            "path": str(rel),
            "standard_id": sid,
            "abi_version": abi or 0,
            "memory": has_memory_export(fpath),
            "simd": has_simd_capabilities(fpath),
            "lines": get_line_count(fpath),
        }
        exports = extract_exports(fpath)
        if exports:
            func_exports = [e["export"] for e in exports if e["kind"] == "func"]
            info["exports"] = func_exports
        modules[mod_name] = info
    return modules


def build_registry():
    registry = {
        "version": 1,
        "generated": datetime.now(timezone.utc).isoformat(),
        "abi_version_current": 2,
        "base_path": "build/wasm/",
        "modules": {}
    }

    categories = sorted([
        d for d in BUILD.iterdir()
        if d.is_dir() and not d.name.startswith(".")
    ])

    for cat_dir in categories:
        cat_name = cat_dir.name
        registry["modules"].update(scan_directory(cat_dir))

    # Also scan ui-framework-wat
    ui_dir = ROOT / "ui-framework-wat"
    if ui_dir.exists():
        for fpath in sorted(ui_dir.glob("*.wat")):
            rel = fpath.relative_to(ROOT)
            mod_name = str(rel.with_suffix(""))
            info = {
                "path": str(rel),
                "standard_id": None,
                "abi_version": 0,
                "memory": has_memory_export(fpath),
                "simd": has_simd_capabilities(fpath),
                "lines": get_line_count(fpath),
            }
            registry["modules"][mod_name] = info

    return registry


def emit_json(registry, path):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(registry, f, indent=2, sort_keys=True)
    print(f"Wrote {path} ({len(registry['modules'])} modules)")


def emit_wat(registry, path):
    """Emit a compact WAT registry module for runtime queries.

    Stores a packed index in memory:
      - Header: count (u32), abi_version_current (u32)
      - Entry per module: standard_id (u32), flags (u32), name_offset (u32), name_len (u32)
      - Name strings packed after entries
    """
    modules = []
    for mod_name, info in registry["modules"].items():
        sid = info.get("standard_id") or 0
        flags = 0
        if info.get("memory"):
            flags |= 1
        if info.get("simd"):
            flags |= 2
        modules.append((sid, flags, mod_name, info["path"]))

    # Build name blob
    name_blob = b""
    name_offsets = []
    for _, _, name, _ in modules:
        name_bytes = name.encode("utf-8")
        name_offsets.append(len(name_blob))
        name_blob += name_bytes + b"\x00"

    # Entry size: 16 bytes (4 u32)
    entry_count = len(modules)
    header_size = 8  # 2 u32
    names_offset = header_size + entry_count * 16

    data_bytes = b""
    # Header
    data_bytes += entry_count.to_bytes(4, "little")
    data_bytes += 2 .to_bytes(4, "little")  # current ABI version
    # Entries
    for i, (sid, flags, _, _) in enumerate(modules):
        off = names_offset + name_offsets[i]
        name_len = len(modules[i][2].encode("utf-8"))
        data_bytes += sid.to_bytes(4, "little")
        data_bytes += flags.to_bytes(4, "little")
        data_bytes += off.to_bytes(4, "little")
        data_bytes += name_len.to_bytes(4, "little")
    # Name blob
    data_bytes += name_blob

    # Format as hex for WAT data section
    hex_str = "".join(f"\\{b:02x}" for b in data_bytes)

    wat = f""";; Generated by build-registry.py — do not edit.
;; Module registry for EdgeRun standards.

(module
  (memory (export "memory") 1)
  (data (i32.const 0) "{hex_str}")

  ;; Header at offset 0:
  ;;   u32 count = number of registered modules
  ;;   u32 abi_version_current = 2
  ;;
  ;; Entry (16 bytes each):
  ;;   u32 standard_id
  ;;   u32 flags  (bit 0 = has_memory, bit 1 = has_simd)
  ;;   u32 name_offset (byte offset from data start)
  ;;   u32 name_len (bytes)
  ;;
  ;; Name strings are packed after entries.

  (func (export "registry_count") (result i32)
    i32.const 0
    i32.load)

  (func (export "registry_abi_version") (result i32)
    i32.const 4
    i32.load)

  (func (export "registry_lookup_by_id")
    (param $sid i32) (result i32)
    (local $count i32) (local $i i32) (local $entry i32) (local $entry_sid i32)
    i32.const 0
    i32.load
    local.set $count
    i32.const 0
    local.set $i
    block $found
      loop $scan
        local.get $i
        local.get $count
        i32.ge_u
        br_if $found
        local.get $i
        i32.const 16
        i32.mul
        i32.const 8
        i32.add
        local.tee $entry
        i32.load
        local.set $entry_sid
        local.get $entry_sid
        local.get $sid
        i32.eq
        if
          local.get $entry
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func (export "registry_lookup_by_name")
    (param $name_ptr i32) (param $name_len i32) (result i32)
    (local $count i32) (local $i i32) (local $entry i32)
    (local $entry_off i32) (local $entry_len i32) (local $j i32) (local $match i32)
    i32.const 0
    i32.load
    local.set $count
    i32.const 0
    local.set $i
    block $found
      loop $scan
        local.get $i
        local.get $count
        i32.ge_u
        br_if $found
        local.get $i
        i32.const 16
        i32.mul
        i32.const 8
        i32.add
        local.tee $entry
        i32.load offset=8
        local.set $entry_off
        local.get $entry
        i32.load offset=12
        local.set $entry_len
        local.get $entry_len
        local.get $name_len
        i32.ne
        if
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $scan
        end
        i32.const 1
        local.set $match
        i32.const 0
        local.set $j
        block $bytes_done
          loop $bytes
            local.get $j
            local.get $name_len
            i32.ge_u
            br_if $bytes_done
            local.get $name_ptr
            local.get $j
            i32.add
            i32.load8_u
            local.get $entry_off
            local.get $j
            i32.add
            i32.load8_u
            i32.ne
            if
              i32.const 0
              local.set $match
              br $bytes_done
            end
            local.get $j
            i32.const 1
            i32.add
            local.set $j
            br $bytes
          end
        end
        local.get $match
        if
          local.get $entry
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan
      end
    end
    i32.const -1)

  (func (export "registry_entry_standard_id") (param $entry i32) (result i32)
    local.get $entry
    i32.load)

  (func (export "registry_entry_simd") (param $entry i32) (result i32)
    local.get $entry
    i32.load offset=4
    i32.const 2
    i32.and)

  (func (export "registry_entry_name_ptr") (param $entry i32) (result i32)
    local.get $entry
    i32.load offset=8)

  (func (export "registry_entry_name_len") (param $entry i32) (result i32)
    local.get $entry
    i32.load offset=12)
)
"""
    with open(path, "w", encoding="utf-8") as f:
        f.write(wat)
    print(f"Wrote {path}")
    return wat


def main():
    parser = argparse.ArgumentParser(description="Build module registry")
    parser.add_argument("--json", default=str(DEFAULT_JSON))
    parser.add_argument("--wat", default=str(DEFAULT_WAT))
    args = parser.parse_args()

    registry = build_registry()
    emit_json(registry, Path(args.json))
    emit_wat(registry, Path(args.wat))

    # Validate the WAT compiles
    result = subprocess.run(
        ["wat2wasm", str(args.wat), "-o", "-"],
        capture_output=True, timeout=30
    )
    if result.returncode == 0:
        print(f"OK — module-registry.wasm compiles ({len(registry['modules'])} entries)")
    else:
        print(f"FAIL — wat2wasm error: {result.stderr.decode()[:200]}", file=sys.stderr)
        sys.exit(1)

    # Count stats
    with_sid = sum(1 for m in registry["modules"].values() if m.get("standard_id"))
    with_simd = sum(1 for m in registry["modules"].values() if m.get("simd"))
    total_lines = sum(m["lines"] for m in registry["modules"].values())
    print(f"Stats: {len(registry['modules'])} modules, {with_sid} with standard_id, "
          f"{with_simd} with SIMD, ~{total_lines} total lines")


if __name__ == "__main__":
    main()
