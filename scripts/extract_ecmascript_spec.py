#!/usr/bin/env python3
"""
Extract ECMAScript (JavaScript) built-in objects, methods, properties, and
grammar productions from the ECMA-262 spec markdown.

Outputs a JSON catalog suitable for Rust code generation.

Usage: python3 scripts/extract_ecmascript_spec.py docs/ecmascript_spec.md > scripts/ecmascript_catalog.json
"""

import json
import re
import sys


def extract_all(markdown_text: str) -> dict:
    """Parse the ECMAScript spec markdown and extract structured data."""
    built_in_objects = []
    global_objects = []
    abstract_operations = []
    grammar_productions = []
    well_known_symbols = []
    well_known_intrinsics = []

    # ---- Built-in object methods/properties ----
    # Pattern: # <span class="secnum">23.1.3.1</span> Array.prototype.at ( `index` )
    method_pattern = re.compile(
        r'# <span class="secnum">([^<]+)</span>\s+'
        r'([A-Z][\w]+)\.prototype\.(\w+)\s*\(\s*([^)]*)\)',
        re.MULTILINE
    )

    # Pattern: # <span class="secnum">23.1.2.1</span> Array ( ...`values` )
    constructor_pattern = re.compile(
        r'# <span class="secnum">([^<]+)</span>\s+'
        r'([A-Z][\w]+)\s*\(\s*([^)]*)\)',
        re.MULTILINE
    )

    # Pattern: # <span class="secnum">23.1.2.2</span> get Array [ %Symbol.species% ]
    accessor_pattern = re.compile(
        r'# <span class="secnum">([^<]+)</span>\s+'
        r'(get|set)\s+([A-Z][\w]+)\[([^\]]+)\]',
        re.MULTILINE
    )

    # Pattern: # <span class="secnum">23.1.2.3</span> Array.prototype.length
    property_pattern = re.compile(
        r'# <span class="secnum">([^<]+)</span>\s+'
        r'([A-Z][\w]+)\.prototype\.(\w+)',
        re.MULTILINE
    )

    # Pattern: # <span class="secnum">23.1.2.3</span> Array.length
    static_property_pattern = re.compile(
        r'# <span class="secnum">([^<]+)</span>\s+'
        r'([A-Z][\w]+)\.(\w+)\s*$',
        re.MULTILINE
    )

    # ---- Abstract operations ----
    # Pattern: # <span class="secnum">7.1.1</span> ToPrimitive ( input \[ , preferredType \] )
    abstract_op_pattern = re.compile(
        r'# <span class="secnum">([^<]+)</span>\s+'
        r'(\w+)\s*\(\s*([^)]*)\)',
        re.MULTILINE
    )

    # ---- Well-known symbols ----
    # Pattern: @@toStringTag, @@species, etc.
    symbol_pattern = re.compile(r'@@([\w]+)')

    # ---- Intrinsic objects ----
    # Pattern: %Array%, %Object%, etc.
    intrinsic_pattern = re.compile(r'%([\w.]+)%')

    # ---- Grammar productions ----
    # Pattern: SourceTextModuleRecord :
    #   ModuleBody
    grammar_pattern = re.compile(
        r'`([A-Z][A-Za-z]+)`\s*:\s*\n\s*`([A-Z][A-Za-z]+)`',
        re.MULTILINE
    )

    # ---- Extract built-in object methods ----
    objects = {}

    for match in method_pattern.finditer(markdown_text):
        sec_num = match.group(1)
        obj_name = match.group(2)
        method_name = match.group(3)
        params = _clean_params(match.group(4))

        if obj_name not in objects:
            objects[obj_name] = {
                "name": obj_name,
                "static_methods": [],
                "prototype_methods": [],
                "static_properties": [],
                "prototype_properties": [],
            }

        # Get the body after the heading
        start = match.end()
        # Find next heading
        next_heading = re.search(r'\n# ', markdown_text[start:start+500])
        end = start + (next_heading.start() if next_heading else min(500, len(markdown_text) - start))
        body = markdown_text[start:start+end-start]

        objects[obj_name]["prototype_methods"].append({
            "name": method_name,
            "parameters": params,
            "section": sec_num,
        })

    # Constructor calls
    for match in constructor_pattern.finditer(markdown_text):
        sec_num = match.group(1)
        obj_name = match.group(2)
        params = _clean_params(match.group(3))

        # Skip if this is actually a method call (has a dot before the paren)
        # Check it's actually a constructor: section number like X.Y.Z where Z is small
        parts = sec_num.split('.')
        if len(parts) >= 2 and int(parts[-1]) <= 10:
            if obj_name not in objects:
                objects[obj_name] = {
                    "name": obj_name,
                    "static_methods": [],
                    "prototype_methods": [],
                    "static_properties": [],
                    "prototype_properties": [],
                }
            # Check if it looks like a constructor (capital letter name, short section)
            if not objects[obj_name].get("constructor_signature"):
                objects[obj_name]["constructor_signature"] = f"{obj_name}({params})"

    # Accessor properties (get/set)
    for match in accessor_pattern.finditer(markdown_text):
        sec_num = match.group(1)
        kind = match.group(2)
        obj_name = match.group(3)
        prop_name = match.group(4).strip().strip('%').strip()

        if obj_name not in objects:
            objects[obj_name] = {
                "name": obj_name,
                "static_methods": [],
                "prototype_methods": [],
                "static_properties": [],
                "prototype_properties": [],
            }
        objects[obj_name]["static_properties"].append({
            "name": prop_name,
            "kind": kind,
            "section": sec_num,
        })

    # Prototype properties
    for match in property_pattern.finditer(markdown_text):
        sec_num = match.group(1)
        obj_name = match.group(2)
        prop_name = match.group(3)

        # Skip if it's actually a method (we already captured those)
        if obj_name in objects:
            existing_methods = {m["name"] for m in objects[obj_name]["prototype_methods"]}
            if prop_name not in existing_methods:
                objects[obj_name]["prototype_properties"].append({
                    "name": prop_name,
                    "section": sec_num,
                })

    # Static properties
    for match in static_property_pattern.finditer(markdown_text):
        sec_num = match.group(1)
        obj_name = match.group(2)
        prop_name = match.group(3)

        # Skip if already captured as accessor or method
        if obj_name in objects:
            existing = {p["name"] for p in objects[obj_name].get("static_properties", [])}
            existing_methods = {m["name"] for m in objects[obj_name].get("prototype_methods", [])}
            if prop_name not in existing and prop_name not in existing_methods:
                # Check it's not a prototype method
                if obj_name in objects:
                    existing_proto_methods = {m["name"] for m in objects[obj_name]["prototype_methods"]}
                    if prop_name not in existing_proto_methods:
                        objects[obj_name]["static_properties"].append({
                            "name": prop_name,
                            "section": sec_num,
                        })

    built_in_objects = list(objects.values())

    # ---- Abstract operations ----
    # These are standalone operations, not methods on objects
    # Pattern: section numbers like 7.x.x or 6.x.x (not 23.x.x which are built-in objects)
    for match in abstract_op_pattern.finditer(markdown_text):
        sec_num = match.group(1)
        name = match.group(2)
        params = _clean_params(match.group(3))

        # Filter to actual abstract operations (section starts with 6 or 7)
        parts = sec_num.split('.')
        if len(parts) >= 1 and parts[0] in ('6', '7'):
            # Skip common false positives
            if name in ('if', 'for', 'while', 'return', 'let', 'const', 'var', 'function', 'class'):
                continue
            abstract_operations.append({
                "name": name,
                "parameters": params,
                "section": sec_num,
            })

    # ---- Well-known symbols ----
    symbols_found = set()
    for match in symbol_pattern.finditer(markdown_text):
        symbols_found.add(match.group(1))
    well_known_symbols = sorted(symbols_found)

    # ---- Well-known intrinsics ----
    intrinsics_found = set()
    for match in intrinsic_pattern.finditer(markdown_text):
        intrinsics_found.add(match.group(1))
    well_known_intrinsics = sorted(intrinsics_found)

    # ---- Grammar productions ----
    for match in grammar_pattern.finditer(markdown_text):
        grammar_productions.append({
            "non_terminal": match.group(1),
            "production": match.group(2),
        })

    # Deduplicate abstract operations
    seen_ops = set()
    unique_ops = []
    for op in abstract_operations:
        if op["name"] not in seen_ops:
            seen_ops.add(op["name"])
            unique_ops.append(op)

    return {
        "spec": "ECMA-262 (ECMAScript)",
        "spec_date": "2026-04-10",
        "total_built_in_objects": len(built_in_objects),
        "total_abstract_operations": len(unique_ops),
        "total_well_known_symbols": len(well_known_symbols),
        "total_well_known_intrinsics": len(well_known_intrinsics),
        "built_in_objects": built_in_objects,
        "abstract_operations": unique_ops,
        "well_known_symbols": well_known_symbols,
        "well_known_intrinsics": well_known_intrinsics,
        "grammar_productions": grammar_productions,
    }


def _clean_params(params_str: str) -> list[str]:
    """Clean parameter string into a list."""
    if not params_str.strip():
        return []
    # Remove backticks and clean
    params_str = params_str.replace('`', '').strip()
    # Remove optional brackets like [ , thisArg ]
    params_str = re.sub(r'\[\s*,?\s*', '[', params_str)
    # Split on commas
    params = [p.strip() for p in params_str.split(',') if p.strip()]
    return params


def main():
    if len(sys.argv) < 2:
        print("Usage: extract_ecmascript_spec.py <path-to-ecmascript-spec.md>", file=sys.stderr)
        sys.exit(1)

    with open(sys.argv[1], 'r', encoding='utf-8') as f:
        markdown_text = f.read()

    catalog = extract_all(markdown_text)
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    main()
