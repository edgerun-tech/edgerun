#!/usr/bin/env python3
"""edgerun monolithic build — merges all modules into one .wasm.

Usage:  python3 system/mbuild.py  →  system/output/edgerun.wasm
"""

import json, os, re, subprocess, sys
from pathlib import Path
from collections import defaultdict

ROOT = Path(__file__).resolve().parent.parent
BUILD_WASM = ROOT / 'build' / 'wasm'
REGISTRY = ROOT / 'registry' / 'module-registry.json'
RUNTIME_CORE = ROOT / 'system' / 'runtime' / 'shared-core.wat'
PORTS = ROOT / 'ports' / 'edgerun-x86-wasm-runtime' / 'wasm-interpreter'
OUTPUT_DIR = ROOT / 'system' / 'output'
OUTPUT_WAT = OUTPUT_DIR / 'edgerun.wat'
OUTPUT_WASM = OUTPUT_DIR / 'edgerun.wasm'

TOTAL_MEMORY_PAGES = 256  # 16 MB


def count_depth(text):
    depth = 0; instr = False; i = 0
    while i < len(text):
        c = text[i]
        if instr:
            if c == '\\': i += 1
            elif c == '"': instr = False
        else:
            if c == '"': instr = True
            elif c == '(': depth += 1
            elif c == ')': depth -= 1
        i += 1
    return depth


def strip_module(text):
    """Remove the outer (module ... ) wrapper, keeping the body intact."""
    # Remove the '(module' token itself (may have inline content)
    text = re.sub(r'^\(module\b', '', text, count=1).lstrip()
    # Remove the LAST ')' in the text — it's always the module's closing paren
    last = text.rfind(')')
    if last >= 0:
        text = text[:last] + text[last+1:]
    return text.strip('\n')


def build():
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    reg = json.load(open(REGISTRY))
    keys = sorted(reg['modules'].keys())
    print(f"Modules: {len(keys)}")

    # Read all modules
    mods = []
    offmap = defaultdict(list)
    for k in keys:
        p = BUILD_WASM / f'{k}.wat'
        if not p.exists(): continue
        t = open(p).read()
        offs = {int(x) for x in re.findall(r'\(data\s+\(i32\.const\s+(\d+)\)', t)}
        for o in offs: offmap[o].append((k, len(mods)))
        mods.append({'key': k, 'text': t, 'offsets': offs})

    print(f"  loaded {len(mods)}")

    # Resolve data offset conflicts
    conflicts = {o: v for o, v in offmap.items() if len(v) > 1}
    print(f"  data offsets: {len(offmap)} unique, {len(conflicts)} conflicting")
    reloc = defaultdict(dict)
    next_off = 0x200000  # well above all stdlib data
    for off, items in sorted(conflicts.items()):
        for key, midx in items:
            reloc[midx][off] = next_off
            next_off += 4096
            next_off = (next_off + 0xFF) & ~0xFF

    # Process each module: strip wrapper, memory, normalize types/names, relocate data
    all_imports = []
    non_import_bodies = []

    def normalize_types(body, midx):
        """Make (type N) unique per module: $t_{midx}_N. Strip (;N;) annotations."""
        # Rename function (;N;) to module-unique names BEFORE stripping
        body = re.sub(r'\(func\s+\(;(\d+);\)', lambda m: f'(func $mod_{midx}_{m.group(1)}', body)
        # Strip remaining (;N;) inline comments
        body = re.sub(r'\(;\d+;\)', '', body)
        # Rename type declarations: (type N) -> (type $t_{midx}_N)
        type_defs = set()
        for m in re.finditer(r'\(type\s+(\d+)\s+\(func', body):
            type_defs.add(int(m.group(1)))
        for tn in sorted(type_defs, reverse=True):
            body = body.replace(f'(type {tn})', f'(type $t_{midx}_{tn})')
        for tn in sorted(type_defs, reverse=True):
            body = body.replace(f'(type {tn} (func', f'(type $t_{midx}_{tn} (func')
        # Build func_map for numeric call rewriting
        func_map = {}
        for m in re.finditer(r'\(func\s+\$mod_' + str(midx) + r'_(\d+)', body):
            func_map[int(m.group(1))] = f'$mod_{midx}_{m.group(1)}'
        # Rewrite numeric calls
        def rewrite_numeric_call(match):
            idx = int(match.group(1))
            if idx in func_map:
                return f'call {func_map[idx]}'
            return match.group(0)
        body = re.sub(r'(?<!\$)call\s+(\d+)\b', rewrite_numeric_call, body)
        # Same for global.get/set N (numeric)
        body = re.sub(r'\bglobal\.(get|set)\s+(\d+)',
                      lambda m: f'global.{m.group(1)} $g_{midx}_{m.group(2)}', body)
        # And call_indirect (type N)
        body = re.sub(r'call_indirect\s+\(type\s+(\d+)\)',
                      lambda m: f'call_indirect (type $t_{midx}_{m.group(1)})', body)
        return body

    for midx, m in enumerate(mods):
        body = strip_module(m['text'])
        key = m['key']
        r = reloc.get(midx, {})

        # Normalize types and names to be module-unique
        body = normalize_types(body, midx)

        # Strip memory
        body = re.sub(r'^\s*\(memory\s+(?:\(export\s+"memory"\))?\s*\d+(?:\s+\d+)?\)\s*\n', '', body, flags=re.MULTILINE)
        body = re.sub(r'^\s*\(export\s+"memory"\s*\(memory\s+\d+\)\)\s*\n', '', body, flags=re.MULTILINE)

        # Extract imports
        imp_lines = []
        other_lines = []
        for ln in body.split('\n'):
            stripped = ln.strip()
            if stripped.startswith('(import '):
                imp_lines.append(stripped)
            else:
                other_lines.append(ln)
        body = '\n'.join(other_lines)

        # Relocate data sections
        for old, new in r.items():
            body = body.replace(f'(data (i32.const {old})', f'(data (i32.const {new})')

        # Fix up i32.const references
        for old, new in sorted(r.items(), key=lambda x: -x[0]):
            body = body.replace(f'i32.const {old}', f'i32.const {new}')

        all_imports.extend(imp_lines)
        non_import_bodies.append((key, body))

    # -- Runtime core --
    rt = open(RUNTIME_CORE).read()
    # Expand LUTs
    lut = subprocess.run(['python3', str(ROOT / 'system' / 'runtime' / 'lut-gen.py')],
                         capture_output=True, text=True).stdout.strip().split('\n')
    rt = rt.replace('${lut_class}', lut[0] if lut else '')
    rt = rt.replace('${lut_lower}', lut[1] if len(lut) > 1 else '')
    rt = re.sub(r'^\s*\(memory\s+(?:\(export\s+"memory"\))?\s*\d+(?:\s+\d+)?\)\s*\n', '', rt, flags=re.MULTILINE)

    imp_lines = []
    other_lines = []
    for ln in rt.split('\n'):
        if ln.strip().startswith('(import '):
            imp_lines.append(ln.strip())
        else:
            other_lines.append(ln)
    all_imports.extend(imp_lines)
    rt_body = '\n'.join(other_lines)

    # -- Interpreter + Compiler --
    def load_port(name):
        text = open(PORTS / name).read()
        body = strip_module(text)
        body = re.sub(r'^\s*\(memory\s+(?:\(export\s+"memory"\))?\s*\d+(?:\s+\d+)?\)\s*\n', '', body, flags=re.MULTILINE)
        # Remove memory imports (we export our own memory)
        body = re.sub(r'^\s*\(import\s+"[^"]*"\s+"memory"\s*\(memory\s+\d+\)\)\s*\n', '', body, flags=re.MULTILINE)
        # Extract other imports
        imp_lines = []
        other_lines = []
        for ln in body.split('\n'):
            s = ln.strip()
            if s.startswith('(import '):
                imp_lines.append(s)
            else:
                other_lines.append(ln)
        return imp_lines, '\n'.join(other_lines)

    itp_imps, itp_body = load_port('interpreter.wat')
    cmp_imps, cmp_body = load_port('compiler.wat')
    all_imports.extend(itp_imps)
    all_imports.extend(cmp_imps)

    # Prefix interpreter/compiler names to avoid collision with runtime core
    def prefix_names(text, prefix):
        # Find all definition names ($xxx)
        names = set()
        for m in re.finditer(r'(?:\(func|\(global)\s+(\$\S+)', text):
            names.add(m.group(1))
        # All references: after call, return_call, global.get, global.set, and as func/global in export
        for m in re.finditer(r'\b(call|return_call|global\.get|global\.set|ref\.func)\s+(\$\S+)', text):
            names.add(m.group(2))
        for m in re.finditer(r'\(export\s+"[^"]*"\s*\((?:func|global)\s+(\$\S+)\)\)', text):
            names.add(m.group(1))
        # Apply longest-first to avoid partial replacements
        for n in sorted(names, key=len, reverse=True):
            text = text.replace(n, f'${prefix}_{n[1:]}')
        return text

    itp_body = prefix_names(itp_body, 'itp')
    cmp_body = prefix_names(cmp_body, 'cmp')

    # -- Assemble --
    lines = ['(module']

    for imp in sorted(set(all_imports)):
        lines.append(f'  {imp}')
    if all_imports:
        lines.append('')

    lines.append(f'  (memory (export "memory") {TOTAL_MEMORY_PAGES})')
    lines.append('')

    # Runtime core
    for ln in rt_body.split('\n'):
        if ln.strip(): lines.append(f'  {ln}')
    lines.append('')

    # Interpreter
    for ln in itp_body.split('\n'):
        if ln.strip(): lines.append(f'  {ln}')
    lines.append('')

    # Compiler
    for ln in cmp_body.split('\n'):
        if ln.strip(): lines.append(f'  {ln}')
    lines.append('')

    # Stdlib modules
    for key, body in non_import_bodies:
        lines.append(f'  ;; -- {key} --')
        for ln in body.split('\n'):
            if ln.strip():
                lines.append(f'  {ln}')

    lines.append(')')
    output = '\n'.join(lines)

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    open(OUTPUT_WAT, 'w').write(output)
    print(f"  wrote {OUTPUT_WAT} ({os.path.getsize(OUTPUT_WAT):,} bytes)")

    print("  compiling...")
    r = subprocess.run(['wat2wasm', str(OUTPUT_WAT), '-o', str(OUTPUT_WASM)],
                       capture_output=True, text=True)
    if r.returncode == 0:
        sz = os.path.getsize(OUTPUT_WASM)
        print(f"  OK: {OUTPUT_WASM} ({sz:,} bytes)")
    else:
        print("  ERRORS:")
        print(r.stderr[:2500])
        import shutil
        shutil.copy(OUTPUT_WAT, OUTPUT_DIR / 'edgerun-debug.wat')
        sys.exit(1)


if __name__ == '__main__':
    build()
