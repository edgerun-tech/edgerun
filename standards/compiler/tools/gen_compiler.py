#!/usr/bin/env python3
"""
EdgeRun JIT Compiler Generator

Reads a per-architecture KV template (JSON) and the unified compiler.wat
template, then generates a complete arch-specific jit-dispatch WAT file.

Usage:
  python tools/gen_compiler.py templates/x86_64.json > gen/jit-dispatch-x86-64.wat
  python tools/gen_compiler.py templates/aarch64.json > gen/jit-dispatch-aarch64.wat
  python tools/gen_compiler.py templates/arm32.json > gen/jit-dispatch-arm32.wat
"""

import json
import sys
import os
import re


def build_op_table(ops, prefix, suffix):
    """Build the opcode dispatch if/eq chain."""
    lines = []
    for hexcode, opname in sorted(ops.items(), key=lambda x: int(x[0], 16)):
        funcname = f"{prefix}{opname}{suffix}"
        code = int(hexcode, 16)
        if code == 0x00:
            lines.append(
                f"          (if (i32.eqz (local.get $opcode))"
                f"\n            (then (call ${funcname} (local.get $dec_ptr)) (br $dispatch_done)))"
            )
        else:
            lines.append(
                f"          (if (i32.eq (local.get $opcode) (i32.const {hexcode}))"
                f"\n            (then (call ${funcname} (local.get $dec_ptr)) (br $dispatch_done)))"
            )
    return "\n".join(lines)


def build_sub_table(ops, prefix, suffix, table_label):
    """Build a sub-opcode dispatch table (for FC/FD prefixes)."""
    lines = []
    for hexcode, opname in sorted(ops.items(), key=lambda x: int(x[0], 16)):
        funcname = f"{prefix}{opname}{suffix}"
        lines.append(
            f"                (if (i32.eq (local.get $imm0) (i32.const {hexcode}))"
            f"\n                  (then (call ${funcname} (local.get $dec_ptr)) (br ${table_label})))"
        )
    return "\n".join(lines)


def generate(tmpl_path, compiler_path):
    """Generate arch-specific dispatch from template KV file."""

    with open(tmpl_path) as f:
        tmpl = json.load(f)

    with open(compiler_path) as f:
        wat = f.read()

    arch = tmpl["arch"]
    suffix = f"_{arch}"
    op_prefix = tmpl["op_prefix"]
    op_suffix = tmpl["op_suffix"]
    simd_prefix = tmpl.get("simd_prefix", op_prefix)
    simd_suffix = tmpl.get("simd_suffix", op_suffix)

    # Build dispatch tables
    op_table = build_op_table(tmpl["ops"], op_prefix, op_suffix)

    fc_ops = tmpl.get("fc_ops", {})
    fc_table = build_sub_table(fc_ops, op_prefix, op_suffix, "fc_done")

    fd_ops = tmpl.get("fd_ops", {})
    fd_table = build_sub_table(fd_ops, simd_prefix, simd_suffix, "fd_done")

    # Substitutions
    subs = {
        "{SUFFIX}": suffix,
        "{OP_TABLE}": op_table,
        "{FC_TABLE}": fc_table,
        "{FD_TABLE}": fd_table,
        "{PROLOGUE}": tmpl["prologue"],
        "{EPILOGUE}": tmpl["epilogue"],
        "{RESULT_GLOBAL}": tmpl["result_global"],
        "{NEXT_OP_GLOBAL}": tmpl["next_op_global"],
        "{JIT_ERROR_GLOBAL}": tmpl["jit_error_global"],
        "{CODE_PTR_GLOBAL}": tmpl["code_ptr_global"],
        "{LABEL_DEPTH_GLOBAL}": tmpl["label_depth_global"],
        "{FUNC_OFF_TABLE}": tmpl["func_off_table"],
        "{ARCH}": arch,
    }

    for placeholder, value in subs.items():
        wat = wat.replace(placeholder, value)

    return wat


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <template.json>", file=sys.stderr)
        sys.exit(1)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    compiler_dir = os.path.normpath(os.path.join(script_dir, ".."))
    compiler_wat = os.path.join(compiler_dir, "compiler.wat")

    if not os.path.exists(compiler_wat):
        print(f"Error: {compiler_wat} not found", file=sys.stderr)
        sys.exit(1)

    result = generate(sys.argv[1], compiler_wat)
    print(result)


if __name__ == "__main__":
    main()
