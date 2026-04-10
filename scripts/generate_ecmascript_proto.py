#!/usr/bin/env python3
"""
Generate protobuf definitions for ECMAScript built-in objects from the catalog JSON.

Usage: python3 scripts/generate_ecmascript_proto.py scripts/ecmascript_catalog.json proto/edgerun/v0/ecmascript/
"""

import json
import os
import re
import sys


def load_catalog(path: str) -> dict:
    with open(path, 'r') as f:
        return json.load(f)


def to_upper_snake(s: str) -> str:
    return re.sub(r'(?<!^)(?=[A-Z])', '_', s).replace('-', '_').upper()


def to_snake_case(s: str) -> str:
    return re.sub(r'(?<!^)(?=[A-Z])', '_', s).replace('-', '_').lower()


def generate_ecmascript_objects_proto(catalog: dict) -> str:
    """Generate ecmascript_objects.proto with built-in object definitions."""
    # Filter to real objects
    objects = [obj for obj in catalog['built_in_objects']
               if obj['prototype_methods'] or obj['static_properties']]

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.ecmascript.objects;',
        '',
        '/// ECMAScript Built-in Object Definitions — generated from ECMA-262.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_ecmascript_proto.py',
        '',

        # ---- JsBuiltInObject enum ----
        '/// All ECMAScript built-in objects.',
        f'/// Total: {len(objects)} objects.',
        'enum JsBuiltInObject {',
        '  JS_BUILT_IN_OBJECT_UNSPECIFIED = 0;',
    ]

    for i, obj in enumerate(objects):
        name = obj['name']
        enum_name = to_upper_snake(name)
        pm = len(obj['prototype_methods'])
        sp = len(obj['static_properties'])
        lines.append(f'  {enum_name} = {i + 1}; // {name} ({pm} methods, {sp} static props)')

    lines.extend([
        '}',
        '',
    ])

    # ---- JsMethod message ----
    lines.extend([
        '/// An ECMAScript built-in method.',
        'message JsMethod {',
        '  string name = 1;',
        '  repeated string parameters = 2;',
        '  string section = 3;',
        '  bool is_prototype_method = 4;',
        '  bool is_static = 5;',
        '}',
        '',
    ])

    # ---- JsProperty message ----
    lines.extend([
        '/// An ECMAScript built-in property.',
        'message JsProperty {',
        '  string name = 1;',
        '  string kind = 2; // "data", "get", "set"',
        '  string section = 3;',
        '}',
        '',
    ])

    # ---- JsBuiltInObjectDef message ----
    lines.extend([
        '/// Complete definition of an ECMAScript built-in object.',
        'message JsBuiltInObjectDef {',
        '  JsBuiltInObject object = 1;',
        '  string name = 2;',
        '  repeated JsMethod prototype_methods = 3;',
        '  repeated JsMethod static_methods = 4;',
        '  repeated JsProperty prototype_properties = 5;',
        '  repeated JsProperty static_properties = 6;',
        '  string section = 7;',
        '}',
        '',
    ])

    return '\n'.join(lines)


def generate_ecmascript_abstract_ops_proto(catalog: dict) -> str:
    """Generate ecmascript_abstract_ops.proto."""
    ops = catalog['abstract_operations']

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.ecmascript.abstract_ops;',
        '',
        '/// ECMAScript Abstract Operation Definitions — generated from ECMA-262.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_ecmascript_proto.py',
        '',

        # ---- JsAbstractOperation enum ----
        '/// All ECMAScript abstract operations.',
        f'/// Total: {len(ops)} operations.',
        'enum JsAbstractOperation {',
        '  JS_ABSTRACT_OPERATION_UNSPECIFIED = 0;',
    ]

    for i, op in enumerate(ops):
        name = op['name']
        enum_name = to_upper_snake(name)
        params = ', '.join(op.get('parameters', []))
        lines.append(f'  {enum_name} = {i + 1}; // {name}({params})')

    lines.extend([
        '}',
        '',

        # ---- JsAbstractOpDef message ----
        '/// Definition of an ECMAScript abstract operation.',
        'message JsAbstractOpDef {',
        '  JsAbstractOperation operation = 1;',
        '  string name = 2;',
        '  repeated string parameters = 3;',
        '  string section = 4;',
        '}',
        '',
    ])

    return '\n'.join(lines)


def generate_ecmascript_globals_proto(catalog: dict) -> str:
    """Generate ecmascript_globals.proto with symbols, intrinsics, and global values."""
    symbols = catalog['well_known_symbols']
    all_intrinsics = catalog['well_known_intrinsics']

    # Deduplicate intrinsics that overlap with symbol names
    symbol_names_upper = {s.upper() for s in symbols}
    intrinsics = []
    seen = set()
    for intrinsic in all_intrinsics:
        key = intrinsic.upper().replace('.', '_')
        if key in seen:
            continue
        seen.add(key)
        # Skip intrinsics that are just symbol references (e.g., %Symbol.iterator%)
        parts = intrinsic.split('.')
        if len(parts) >= 2 and parts[0] == 'Symbol' and parts[-1].upper() in symbol_names_upper:
            continue
        intrinsics.append(intrinsic)

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.ecmascript.globals;',
        '',
        '/// ECMAScript Global Symbols and Intrinsics — generated from ECMA-262.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_ecmascript_proto.py',
        '',

        # ---- JsWellKnownSymbol enum ----
        '/// ECMAScript well-known symbols (@@iterator, @@toStringTag, etc.).',
        f'/// Total: {len(symbols)} symbols.',
        'enum JsWellKnownSymbol {',
        '  JS_WELL_KNOWN_SYMBOL_UNSPECIFIED = 0;',
    ]

    for i, sym in enumerate(symbols):
        enum_name = 'SYMBOL_' + to_upper_snake(sym)
        lines.append(f'  {enum_name} = {i + 1}; // @@{sym}')

    lines.extend([
        '}',
        '',

        # ---- JsIntrinsic enum ----
        '/// ECMAScript well-known intrinsic objects (%Array%, %Object%, etc.).',
        f'/// Total: {len(intrinsics)} intrinsics.',
        'enum JsIntrinsic {',
        '  JS_INTRINSIC_UNSPECIFIED = 0;',
    ])

    for i, intrinsic in enumerate(intrinsics):
        # Skip very long intrinsic names
        if len(intrinsic) > 80:
            continue
        enum_name = 'INTRINSIC_' + to_upper_snake(intrinsic.replace('.', '_'))
        # Proto enum values must start with a letter
        if enum_name[0].isdigit():
            enum_name = 'I' + enum_name
        lines.append(f'  {enum_name} = {i + 1}; // %{intrinsic}%')

    lines.extend([
        '}',
        '',
    ])

    return '\n'.join(lines)


def main():
    if len(sys.argv) < 3:
        print("Usage: generate_ecmascript_proto.py <catalog.json> <output_dir>", file=sys.stderr)
        sys.exit(1)

    catalog = load_catalog(sys.argv[1])
    output_dir = sys.argv[2]
    os.makedirs(output_dir, exist_ok=True)

    files = {
        'ecmascript_objects.proto': generate_ecmascript_objects_proto(catalog),
        'ecmascript_abstract_ops.proto': generate_ecmascript_abstract_ops_proto(catalog),
        'ecmascript_globals.proto': generate_ecmascript_globals_proto(catalog),
    }

    for filename, content in files.items():
        path = os.path.join(output_dir, filename)
        with open(path, 'w') as f:
            f.write(content)
        print(f"Generated: {path}")

    print(f"\nDone. {len(files)} proto files generated.")


if __name__ == "__main__":
    main()
