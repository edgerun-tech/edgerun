#!/usr/bin/env python3
"""
Extract CSS property definitions from W3C/WHATWG CSS spec markdown files.
Outputs a JSON catalog suitable for Rust code generation.

Usage: python3 scripts/extract_css_specs.py docs/w3c_specs/ > scripts/css_property_catalog.json
"""

import json
import os
import re
import sys
from collections import defaultdict


def extract_properties_from_file(filepath: str) -> list[dict]:
    """Extract CSS property definitions from a single spec markdown file."""
    with open(filepath, 'r', encoding='utf-8') as f:
        text = f.read()

    properties = []
    # Find all "Name:" lines that start a property definition block
    # Pattern: Name:\n\n<span...dfn-type="property"...>property-name</span>\n\nValue:\n...
    name_pattern = re.compile(
        r'^Name:\s*\n+'
        r'(?:.*?\n)*?'
        r'dfn-type="property"[^>]*>([^<]+)</span>',
        re.MULTILINE
    )

    for match in name_pattern.finditer(text):
        prop_name = match.group(1).strip()
        start = match.start()

        # Find the end (next "Name:" or end of file)
        next_match = name_pattern.search(text, match.end())
        end = next_match.start() if next_match else len(text)
        block = text[start:end]

        prop = {
            "name": prop_name,
            "value": "",
            "initial": "",
            "applies_to": "",
            "inherited": "",
            "percentages": "",
            "computed_value": "",
            "animation_type": "",
            "canonical_order": "",
            "source_file": os.path.basename(filepath),
        }

        # Extract each field
        for field in ("value", "initial", "applies_to", "inherited", "percentages",
                       "computed_value", "animation_type", "canonical_order"):
            # Handle both "Value:" and "[Value:](...)" patterns
            field_re = re.compile(
                r'(?:\[?' + re.escape(field.capitalize().replace("_", " ")) + r':\]?\s*(?:\([^)]*\))?\s*\n+)'
                r'(.*?)(?=\n+\[?[A-Z][\w ]+:\]?\s*(?:\([^)]*\))?\s*\n|\Z)',
                re.DOTALL
            )
            m = field_re.search(block)
            if m:
                raw = m.group(1).strip()
                prop[field] = _strip_html(raw)

        properties.append(prop)

    return properties


def extract_value_types_from_css_values(filepath: str) -> list[dict]:
    """Extract CSS value type definitions from css-values spec."""
    with open(filepath, 'r', encoding='utf-8') as f:
        text = f.read()

    value_types = []

    # Pattern: dfn-type="value" or dfn-type="type"
    value_pattern = re.compile(
        r'dfn-type="(value|type)"[^>]*>'
        r'(?:&lt;)?([^<]+?)(?:&gt;)?</span>',
        re.MULTILINE
    )

    for match in value_pattern.finditer(text):
        vtype = match.group(1)
        name = match.group(2).strip().strip('<>').strip()
        if not name or name.lower() in ('css', 'the', 'a', 'an'):
            continue
        value_types.append({
            "name": name,
            "kind": vtype,
        })

    # Deduplicate
    seen = set()
    unique = []
    for vt in value_types:
        key = vt["name"].lower()
        if key not in seen:
            seen.add(key)
            unique.append(vt)

    return unique


def extract_at_rules(filepath: str) -> list[dict]:
    """Extract CSS at-rule definitions."""
    with open(filepath, 'r', encoding='utf-8') as f:
        text = f.read()

    at_rules = []
    # Pattern: dfn-type="at-rule"
    at_rule_pattern = re.compile(
        r'dfn-type="at-rule"[^>]*>@?([^<]+)</span>'
    )

    for match in at_rule_pattern.finditer(text):
        name = match.group(1).strip().lstrip('@')
        at_rules.append({"name": name})

    return at_rules


def _strip_html(text: str) -> str:
    """Remove HTML tags and collapse whitespace."""
    text = re.sub(r'<[^>]+>', '', text)
    text = text.replace('&lt;', '<')
    text = text.replace('&gt;', '>')
    text = text.replace('&amp;', '&')
    text = text.replace('&#39;', "'")
    text = text.replace('&quot;', '"')
    text = text.replace('&nbsp;', ' ')
    text = re.sub(r'\s+', ' ', text).strip()
    # Remove reference links like [1], [2]
    text = re.sub(r'\[\d+\]', '', text)
    return text


def main():
    if len(sys.argv) < 2:
        print("Usage: extract_css_specs.py <specs_dir>", file=sys.stderr)
        sys.exit(1)

    specs_dir = sys.argv[1]

    all_properties = []
    all_value_types = []
    all_at_rules = []

    css_files = [
        f for f in sorted(os.listdir(specs_dir))
        if f.endswith('.md') and (
            f.startswith('drafts.csswg.org_css') or
            f.startswith('www.w3.org_TR_css') or
            f.startswith('drafts.csswg.org_mediaqueries')
        )
    ]

    for filename in css_files:
        filepath = os.path.join(specs_dir, filename)
        props = extract_properties_from_file(filepath)
        all_properties.extend(props)

        if 'css-values' in filename:
            vtypes = extract_value_types_from_css_values(filepath)
            all_value_types.extend(vtypes)

        at_rules = extract_at_rules(filepath)
        all_at_rules.extend(at_rules)

    # Deduplicate properties by name (keep first occurrence)
    seen_props = set()
    unique_props = []
    for p in all_properties:
        if p["name"].lower() not in seen_props:
            seen_props.add(p["name"].lower())
            unique_props.append(p)

    # Deduplicate at-rules by name
    seen_at_rules = set()
    unique_at_rules = []
    for r in all_at_rules:
        if r["name"].lower() not in seen_at_rules:
            seen_at_rules.add(r["name"].lower())
            unique_at_rules.append(r)

    catalog = {
        "spec": "W3C CSS Specifications",
        "spec_date": "2026-04-10",
        "total_properties": len(unique_props),
        "total_value_types": len(all_value_types),
        "total_at_rules": len(unique_at_rules),
        "properties": unique_props,
        "value_types": all_value_types,
        "at_rules": unique_at_rules,
    }

    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    main()
