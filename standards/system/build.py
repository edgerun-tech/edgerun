#!/usr/bin/env python3
"""
Unified edgerun system build pipeline.

Reads all WAT modules from the registry, links them into a single
megamodule with shared runtime core, character classification LUTs,
SIMD stubs, and a dispatch table.

Usage:
    python3 system/build.py [options]

Output:
    system/output/edgerun-system.wat
    system/output/edgerun-system.wasm
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILD_WASM = ROOT / 'build' / 'wasm'
REGISTRY = ROOT / 'registry' / 'module-registry.json'
RUNTIME_CORE = ROOT / 'system' / 'runtime' / 'shared-core.wat'
OUTPUT_DIR = ROOT / 'system' / 'output'
OUTPUT_WAT = OUTPUT_DIR / 'edgerun-system.wat'
OUTPUT_WASM = OUTPUT_DIR / 'edgerun-system.wasm'

# Known shared helper function names
SHARED_HELPERS = {
    'char_class', 'is_digit', 'is_upper', 'is_lower', 'is_alpha', 'is_tchar',
    'is_hex', 'is_ws', 'is_scheme_byte', 'is_label_byte', 'to_lower',
    'pack', 'pack_u16', 'byte', 'has', 'is_cont',
    'emit_byte', 'emit_dword', 'emit_modrm',
    'simd_memchr', 'simd_memrchr',
}

SHARED_HELPER_PATTERN = re.compile(
    r'^\$(' + '|'.join(sorted(SHARED_HELPERS, key=len, reverse=True)) + r')\b'
)

# ABI exports that shouldn't be re-exported from individual modules
ABI_EXPORTS = {
    'proto_abi_version', 'proto_standard_id', 'simd_capabilities', 'memory',
}

# Patterns
RE_FUNC_DECL = re.compile(r'^\s*\(func\s(\$\S+)?')
RE_TYPE_DECL = re.compile(r'^\s*\(type\s+(\$\S+|\d+)\s*\(func')
RE_EXPORT = re.compile(r'^\s*\(export\s+"([^"]*)"\s*\(func\s+(\$\S+)\)\)\s*$')
RE_EXPORT_MEM = re.compile(r'^\s*\(export\s+"memory"\s*\(memory\s+\d+\)\)\s*$')
RE_TYPE_REF = re.compile(r'\(type\s+(\$\S+|\d+)\)')

# Global counters (spans all modules)
# Using lists so they're mutable without global keyword issues
_anon_counter = [0]  # unique anonymous function ids
_module_counter = [0]  # unique module ids


def load_registry():
    with open(REGISTRY) as f:
        return json.load(f)


def read_wat(filepath):
    with open(filepath) as f:
        return f.read()


def parse_type_section(wat_text):
    """Parse type definitions from WAT text.
    Returns: {type_index: (params, results), type_name: (params, results)}
    where params and results are lists like ['i32', 'i32'].
    """
    types = {}
    named_types = {}
    lines = wat_text.split('\n')

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        m = RE_TYPE_DECL.match(stripped)
        if m:
            type_ref = m.group(1)
            # Parse the full type declaration, collecting innards
            depth = 1
            body_parts = []
            # If this line contains the full type:
            if stripped.rstrip().endswith(')'):
                body = stripped
            else:
                body_parts.append(stripped)
                i += 1
                while i < len(lines) and depth > 0:
                    s = lines[i].strip()
                    depth += s.count('(') - s.count(')')
                    body_parts.append(s)
                    i += 1
                body = ''.join(body_parts)

            # Extract params and results
            params = re.findall(r'\(param\s+([\w\s]*(?:i32|i64|f32|f64|v128)(?:\s+\$?\w+)*)\)', body)
            results = re.findall(r'\(result\s+([\w\s]*(?:i32|i64|f32|f64|v128)(?:\s+\$?\w+)*)\)', body)

            param_types = []
            for p in params:
                parts = p.split()
                for part in parts:
                    if part in ('i32', 'i64', 'f32', 'f64', 'v128'):
                        param_types.append(part)

            result_types = []
            for r in results:
                parts = r.split()
                for part in parts:
                    if part in ('i32', 'i64', 'f32', 'f64', 'v128'):
                        result_types.append(part)

            if type_ref.startswith('$'):
                named_types[type_ref] = (param_types, result_types)
            else:
                types[int(type_ref)] = (param_types, result_types)

            continue

        i += 1

    # Merge: numeric indices override named (they occupy the type section slots)
    all_types = {}
    all_types.update(types)
    # Also include named types
    for name, sig in named_types.items():
        all_types[name] = sig

    return all_types, types, named_types


def expand_type_refs(func_text, type_map, type_by_num, type_by_name):
    """Given a function declaration, if it uses (type N), expand to explicit params/results."""
    m = RE_TYPE_REF.search(func_text)
    if not m:
        return func_text

    type_ref = m.group(1)

    # Look up the type signature
    sig = None
    if type_ref.startswith('$'):
        sig = type_by_name.get(type_ref)
    else:
        sig = type_by_num.get(int(type_ref))

    if not sig:
        return func_text  # Can't expand, leave as-is

    param_types, result_types = sig

    # Remove the (type ...) reference from the function declaration
    result = func_text.replace(m.group(0), '', 1)

    # Add explicit params and results if not already present
    if param_types:
        param_str = ' '.join(f'(param {t})' for t in param_types)
        if '(param' not in result:
            result = result.replace('(func ', f'(func {param_str} ', 1)
        # else: already has params, keep them

    if result_types:
        result_str = ' '.join(f'(result {t})' for t in result_types)
        if '(result' not in result:
            result = result.replace('(func ', f'(func {result_str} ', 1) if '(param' not in result \
                else result.replace('(param ', f'{result_str} (param ', 1)  # crude, improve later
        # else: already has results, keep them

    return result


def strip_types(wat_text):
    """Remove all (type ...) declarations."""
    lines = wat_text.split('\n')
    result = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith('(type ') or stripped.startswith('(type\t'):
            continue
        result.append(line)
    return '\n'.join(result)


def count_str_aware_depth(text):
    """Count paren depth, correctly skipping string literals and escape sequences."""
    depth = 0
    in_str = False
    i = 0
    while i < len(text):
        c = text[i]
        if in_str:
            if c == '\\':
                i += 1  # skip escaped char
            elif c == '"':
                in_str = False
        else:
            if c == '"':
                in_str = True
            elif c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
        i += 1
    return depth


def strip_module_header(wat_text):
    """Remove the (module ... header and final closing paren from wasm2wat output.

    Handles the case where the module's closing paren is on the same line
    as the last construct (e.g., data section). Uses string-aware paren
    counting to avoid false positives from raw parens in data strings.
    """
    lines = wat_text.split('\n')
    result = []
    module_depth = 0
    in_module = False

    for line in lines:
        stripped = line.strip()
        if not in_module:
            if stripped.startswith('(module'):
                in_module = True
                module_depth = count_str_aware_depth(stripped)
                if module_depth <= 0:
                    in_module = False
                continue
        else:
            line_depth = count_str_aware_depth(stripped)
            new_depth = module_depth + line_depth

            if new_depth > 0:
                module_depth = new_depth
                result.append(line)
            elif new_depth == 0:
                # This line closes the module: it has line_depth extra )'s.
                # Remove those trailing )'s and keep the rest as module content.
                # line_depth is negative (e.g., -1, -2), so excess = -line_depth
                excess = -line_depth  # number of module-closing parens on this line
                trimmed = line
                for _ in range(excess):
                    last_paren = trimmed.rfind(')')
                    if last_paren >= 0:
                        trimmed = trimmed[:last_paren] + trimmed[last_paren+1:]
                if trimmed.strip():
                    result.append(trimmed)
                module_depth = 0
                in_module = False
            else:
                # new_depth < 0 means we somehow overshot
                module_depth = 0
                in_module = False

    return '\n'.join(result)


def normalize_module(wat_text, tmp_dir='/tmp/wat_normalize'):
    """Compile to .wasm and decompile back to canonical WAT.
    Handles string escaping properly and eliminates type references.
    Returns (canonical_wat, anonymous_func_count) or (original_wat, 0) on failure."""
    os.makedirs(tmp_dir, exist_ok=True)
    tmp_input = os.path.join(tmp_dir, 'input.wat')
    tmp_output = os.path.join(tmp_dir, 'output.wasm')

    with open(tmp_input, 'w') as f:
        f.write(wat_text)

    result = subprocess.run(
        ['wat2wasm', tmp_input, '-o', tmp_output],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"  normalize FAIL (compile): {result.stderr[:200]}")
        return None

    result = subprocess.run(
        ['wasm2wat', tmp_output],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        return None

    return result.stdout


def transform_module(wat_text, sid, mid, module_key, module_info):
    """Transform a single module for inclusion in the megamodule.

    1. Normalizes via wasm2wat (handles string escaping, expands types)
    2. Strips type/memory/import/table/global declarations
    3. Names anonymous functions
    4. Removes shared helper function definitions
    5. Prefixes non-shared function names with $mod_{sid}_
    6. Prefixes export names with {sid}_
    7. Rewrites internal function references (including numeric calls)
    """
    # Normalize via wasm2wat
    canonical = normalize_module(wat_text)
    if canonical is None:
        print(f"  SKIPPING {module_key}: failed normalization (WAT syntax error)")
        return None
    lines = canonical.split('\n')

    # Remove module header using string-aware paren counting
    content = strip_module_header(canonical)
    content_lines = content.split('\n')

    # First pass: find all function declarations and exports
    func_names = {}      # func_index -> old_name (None if anonymous)
    export_to_func = {}  # export_name -> old_func_name
    anon_count = 0
    func_count = 0
    func_explicit_names = {}  # numeric index of func in original module -> explicit name
    type_refs = {}       # func_index -> type_index

    i = 0
    while i < len(content_lines):
        stripped = content_lines[i].strip()
        if not stripped:
            i += 1
            continue

        if stripped.startswith('(func'):
            # Parse function declaration
            # wasm2wat output: (func (;N;) (type M) (param ...) (result ...) ...
            # or: (func $name (type M) (param ...) (result ...) ...
            explicit_name = None
            ftype = None
            func_bin_idx = None  # index from (;N;) comment

            # Check for explicit name: (func $name ...
            m_name = re.match(r'^\s*\(func\s+(\$\S+)\b', stripped)
            if m_name:
                explicit_name = m_name.group(1)

            # Check for (;N;) binary index comment
            m_anon_idx = re.match(r'^\s*\(func\s+\(;(\d+);\)', stripped)
            if m_anon_idx:
                func_bin_idx = int(m_anon_idx.group(1))

            # Check for type reference
            m_type = re.search(r'\(type\s+(\d+)\)', stripped)
            if m_type:
                ftype = int(m_type.group(1))

            func_explicit_names[func_count] = {
                'name': explicit_name or f'__anon_{anon_count}',
                'bin_idx': func_bin_idx,
            }
            if explicit_name is None:
                anon_count += 1
            if ftype is not None:
                type_refs[func_count] = ftype
            func_count += 1

        elif stripped.startswith('(export "'):
            m = RE_EXPORT.match(stripped)
            if m:
                export_to_func[m.group(1)] = m.group(2)

        i += 1

    # Build function info
    all_funcs = set()
    shared_funcs = set()
    for idx, info in func_explicit_names.items():
        name = info['name']
        if name.startswith('$'):
            all_funcs.add(name)
            if SHARED_HELPER_PATTERN.match(name):
                shared_funcs.add(name)

    # Build rename map using globally unique anonymous function names
    renames = {}
    for idx, info in func_explicit_names.items():
        name = info['name']
        if name.startswith('$') and name not in shared_funcs:
            renames[name] = f'$mod_{sid}_{name[1:]}'
        elif not name.startswith('$'):
            # Anonymous function: use globally unique number
            renames[name] = f'$anon_{_anon_counter[0]}'
            _anon_counter[0] += 1

    # Build func_map: maps (;N;) binary index to new name for anonymous functions
    # Named functions use func_count as key for numeric call rewriting
    func_map_bin_idx = {}  # bin_idx -> new_name (for anonymous with (;N;))
    func_map_named = {}    # func_count -> new_name (for named functions)
    for idx, info in func_explicit_names.items():
        name = info['name']
        bin_idx = info['bin_idx']
        if name in renames:
            if bin_idx is not None:
                func_map_bin_idx[bin_idx] = renames[name]
            else:
                func_map_named[idx] = renames[name]
        elif name.startswith('$') and name in shared_funcs:
            func_map_named[idx] = name  # shared, keep original name
        else:
            func_map_named[idx] = name

    # Second pass: rewrite
    # We apply call rewriting to ALL lines (function bodies, data, etc.)
    # After wasm2wat, calls come in two forms:
    #   - call $name for named functions
    #   - call N for anonymous functions (using binary index)
    # We handle return_call the same way.

    output_lines = []
    skip_depth = 0
    in_func = False

    def rewrite_line(line):
        """Rewrite call/return_call/global.get/global.set references in a line."""
        # handle call $name
        def rewrite_call(match):
            name = match.group(1)
            if name in renames:
                return f'call {renames[name]}'
            return match.group(0)

        result = re.sub(r'call\s+(\$\S+)', rewrite_call, line)

        # handle call N (numeric index - anonymous, using binary index)
        def rewrite_numeric_call(match):
            idx = int(match.group(1))
            if idx in func_map_bin_idx:
                new_name = func_map_bin_idx[idx]
                if new_name.startswith('$'):
                    return f'call {new_name}'
            return match.group(0)

        result = re.sub(r'(?<!\$)(?<!\w)call\s+(\d+)(?!\w)', rewrite_numeric_call, result)

        # handle return_call $name
        def rewrite_return_call(match):
            name = match.group(1)
            if name in renames:
                return f'return_call {renames[name]}'
            return match.group(0)

        result = re.sub(r'return_call\s+(\$\S+)', rewrite_return_call, result)

        # handle global.get/set N (numeric global index)
        def rewrite_global_get(match):
            idx = int(match.group(2))
            return f'global.{match.group(1)} $g_{mid}_{idx}'

        result = re.sub(r'\bglobal\.(get|set)\s+(\d+)\b', rewrite_global_get, result)

        # handle call_indirect (type N) - rewrite type reference
        result = re.sub(r'call_indirect\s+\(type\s+(\d+)\)',
                        lambda m: f'call_indirect (type $t_{mid}_{m.group(1)})',
                        result)
        return result

    for line in content_lines:
        stripped = line.strip()
        if not stripped:
            output_lines.append('')
            continue

        # Keep type declarations but rename them to avoid collisions
        if stripped.startswith('(type '):
            line = re.sub(
                r'\(;(\d+);\)',
                lambda m: f'$t_{mid}_{m.group(1)}',
                stripped
            )
            output_lines.append(f'  {line}')
            continue

        # Skip memory, import
        if (stripped.startswith('(memory ') or stripped.startswith('(memory\t') or
            stripped.startswith('(import ') or stripped.startswith('(import\t')):
            continue

        # Skip (export "memory" ...)
        if RE_EXPORT_MEM.match(stripped):
            continue
        if stripped.startswith('(export "memory"'):
            continue

        # Handle global declarations: rename (;N;) to $g_{mid}_{N}
        if stripped.startswith('(global '):
            line = re.sub(
                r'\(;(\d+);\)',
                lambda m: f'$g_{mid}_{m.group(1)}',
                stripped
            )
            output_lines.append(f'  {line}')
            continue

        # Handle function declarations
        if stripped.startswith('(func'):
            m_name = re.match(r'^\s*\(func\s+(\$\S+)\b', stripped)
            explicit_name = m_name.group(1) if m_name else None

            if explicit_name and explicit_name in shared_funcs:
                skip_depth = 1
                if stripped.rstrip().endswith(')'):
                    skip_depth = 0
                continue

            # Strip (type N) reference
            line = re.sub(r'\s*\(type\s+\d+\)', '', stripped)

            # Replace anonymous function name with explicit name
            if not explicit_name:
                m_anon = re.match(r'^\s*\(func\s+\(;(\d+);\)', line)
                if m_anon:
                    fidx = int(m_anon.group(1))
                    new_name = func_map_bin_idx.get(fidx)
                    if new_name is None:
                        new_name = f'$anon_{fidx}'
                    line = re.sub(r'\(;\d+;\)', new_name, line, count=1)
            elif explicit_name in renames:
                line = line.replace(explicit_name, renames[explicit_name], 1)

            line = rewrite_line(line)
            output_lines.append(f'  {line}')
            continue

        # Handle exports (from ABI, rename and re-export)
        if stripped.startswith('(export "'):
            m = RE_EXPORT.match(stripped)
            if m:
                export_name = m.group(1)
                if export_name in ABI_EXPORTS:
                    continue
                func_ref = m.group(2)
                new_ref = renames.get(func_ref, func_ref)
                output_lines.append(f'  (export "{sid}_{export_name}" (func {new_ref}))')
            continue

        # For other constructs (data, comments, function body lines, etc.),
        # apply call rewriting and pass through
        line = rewrite_line(line)
        output_lines.append(f'  {line}')
        continue

        # Handle exports (from ABI, rename and re-export)
        if stripped.startswith('(export "'):
            m = RE_EXPORT.match(stripped)
            if m:
                export_name = m.group(1)
                if export_name in ABI_EXPORTS:
                    continue
                func_ref = m.group(2)
                new_ref = renames.get(func_ref, func_ref)
                output_lines.append(f'  (export "{sid}_{export_name}" (func {new_ref}))')
            continue

        # For other constructs (data, comments, etc.), just pass through
        output_lines.append(f'  {line}')

    return '\n'.join(output_lines)


def build_dispatch_table(modules):
    """Generate the dispatch WAT table."""
    return '''  ;; Unified dispatch table placeholder
  (func (export "edgerun_dispatch")
    (param $sid i32) (param $func_idx i32)
    (param $args_ptr i32) (param $args_len i32)
    (param $result_ptr i32) (param $result_len i32)
    (result i32)
    (i32.const 0))
'''


def write_megamodule(modules, output_path):
    """Write the unified megamodule WAT file."""
    with open(RUNTIME_CORE) as f:
        runtime_core = f.read()

    # Generate LUT data
    lut_gen = ROOT / 'system' / 'runtime' / 'lut-gen.py'
    result = subprocess.run(
        ['python3', str(lut_gen)],
        capture_output=True, text=True
    )
    lut_lines = result.stdout.strip().split('\n')
    lut_class_data = lut_lines[0] if len(lut_lines) > 0 else ''
    lut_lower_data = lut_lines[1] if len(lut_lines) > 1 else ''

    runtime_core = runtime_core.replace('${lut_class}', lut_class_data)
    runtime_core = runtime_core.replace('${lut_lower}', lut_lower_data)

    with open(output_path, 'w') as f:
        f.write('(module\n\n')
        f.write('  ;; ════════════════════════════════════════════════════\n')
        f.write('  ;;  edgerun unified system module\n')
        f.write('  ;;  Generated by system/build.py\n')
        f.write(f'  ;;  Contains {len(modules)} modules\n')
        f.write('  ;; ════════════════════════════════════════════════════\n\n')

        # Write runtime core
        for line in runtime_core.split('\n'):
            if line.strip():
                f.write(f'  {line}\n')
            else:
                f.write('\n')

        f.write('\n  ;; ════════════════════════════════════════════════════\n')
        f.write(f'  ;;  {len(modules)} linked modules\n')
        f.write('  ;; ════════════════════════════════════════════════════\n\n')

        # Write each transformed module
        for key, mod_info in modules:
            f.write(f'  ;; -- {mod_info["display_name"]} (sid={mod_info["sid"]}) --\n')
            f.write(f'  ;;  source: {mod_info["filepath"]}\n\n')
            mod_text = mod_info['transformed']
            # Ensure proper indentation
            for line in mod_text.split('\n'):
                if line.strip():
                    f.write(f'  {line}\n')
                else:
                    f.write('\n')
            f.write('\n')

        # Write dispatch table
        f.write(build_dispatch_table(modules))
        f.write(')\n')

    print(f"  wrote {output_path}")


def build():
    registry = load_registry()
    print(f"Building unified edgerun system...")
    print(f"  registry: {len(registry['modules'])} modules")

    modules = []
    module_keys = sorted(registry['modules'].keys())

    for key in module_keys:
        info = registry['modules'][key]
        filepath = BUILD_WASM / f'{key}.wat'

        if not filepath.exists():
            continue

        wat = read_wat(filepath)
        sid = info.get('standard_id', 0)
        if not sid:
            sid = 0
        sid = int(sid)

        mid = _module_counter[0]
        _module_counter[0] += 1
        transformed = transform_module(wat, sid, mid, key, info)
        if transformed is None:
            continue
        display_name = key.rsplit('/', 1)[-1].rsplit('.', 1)[0]
        modules.append((key, {
            'key': key,
            'sid': sid,
            'display_name': display_name,
            'filepath': str(filepath),
            'simd': info.get('simd', False),
            'lines': info.get('lines', 0),
            'transformed': transformed,
        }))

        sys.stdout.write(f"\r  linking: {len(modules)}/{len(module_keys)}")
        sys.stdout.flush()

    print()
    print(f"  total linked: {len(modules)} modules")

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    write_megamodule(modules, OUTPUT_WAT)

    # Compile to WASM
    wat_size = os.path.getsize(OUTPUT_WAT)
    print(f"  WAT size: {wat_size:,} bytes")

    print("  compiling to WASM...")
    result = subprocess.run(
        ['wat2wasm', str(OUTPUT_WAT), '-o', str(OUTPUT_WASM)],
        capture_output=True, text=True
    )
    if result.returncode == 0:
        wasm_size = os.path.getsize(OUTPUT_WASM)
        print(f"  compiled: {OUTPUT_WASM} ({wasm_size:,} bytes)")
        print(f"  ratio: {wasm_size / wat_size:.2%}")
        print("  DONE")
    else:
        print(f"  COMPILE ERROR:")
        stderr = result.stderr
        if len(stderr) > 2000:
            print(stderr[:2000])
            print("  ... (truncated)")
        else:
            print(stderr)
        debug_path = OUTPUT_DIR / 'edgerun-system-debug.wat'
        import shutil
        shutil.copy(OUTPUT_WAT, debug_path)
        print(f"  WAT saved to {debug_path} for debugging")
        sys.exit(1)


if __name__ == '__main__':
    build()
