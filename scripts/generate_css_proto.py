#!/usr/bin/env python3
"""
Generate protobuf definitions for CSS properties from the catalog JSON.

Usage: python3 scripts/generate_css_proto.py scripts/css_property_catalog.json proto/edgerun/v0/css/
"""

import json
import os
import sys


def load_catalog(path: str) -> dict:
    with open(path, 'r') as f:
        return json.load(f)


def to_upper_snake(s: str) -> str:
    return s.replace('-', '_').upper()


def to_snake_case(s: str) -> str:
    return s.replace('-', '_')


def infer_proto_type(value_syntax: str) -> str:
    """Infer a proto type from CSS value syntax."""
    v = value_syntax.lower().strip()
    if v.startswith('<color'):
        return 'string'
    if v.startswith('<length') or v.startswith('<percentage') or v.startswith('<dimension'):
        return 'string'
    if v.startswith('<integer') or v.startswith('<number'):
        return 'double'
    if v.startswith('<time'):
        return 'double'
    if v.startswith('<angle'):
        return 'double'
    if v.startswith('<frequency'):
        return 'double'
    if v.startswith('<resolution'):
        return 'double'
    if 'none' == v or v.startswith('none '):
        return 'string'
    if v in ('auto', 'inherit', 'initial', 'unset', 'revert'):
        return 'string'
    if v.startswith('bool') or v.startswith('true') or v.startswith('false'):
        return 'bool'
    return 'string'


def generate_css_properties_proto(catalog: dict) -> str:
    """Generate css_properties.proto with all CSS property definitions."""
    properties = catalog['properties']

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.css.properties;',
        '',
        '/// CSS Property Definitions — generated from W3C CSS specifications.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_css_proto.py',
        '',

        # ---- CssProperty enum ----
        '/// All CSS properties defined in W3C CSS specifications.',
        f'/// Total: {len(properties)} properties.',
        'enum CssProperty {',
        '  CSS_PROPERTY_UNSPECIFIED = 0;',
    ]

    for i, prop in enumerate(properties):
        name = prop['name']
        enum_name = to_upper_snake(name)
        desc = prop.get('value', '').replace('`', '').replace('"', '\\"')
        if desc and len(desc) < 120:
            lines.append(f'  // {name}: {desc}')
        lines.append(f'  {enum_name} = {i + 1};')

    lines.extend([
        '}',
        '',
    ])

    # ---- PropertyDefinition message ----
    lines.extend([
        '/// Complete definition of a CSS property.',
        'message CssPropertyDefinition {',
        '  CssProperty property = 1;',
        '  string name = 2;',
        '  string value_syntax = 3;',
        '  string initial_value = 4;',
        '  string applies_to = 5;',
        '  bool inherited = 6;',
        '  string percentages = 7;',
        '  string computed_value = 8;',
        '  string animation_type = 9;',
        '  string canonical_order = 10;',
        '  string source_spec = 11;',
        '}',
        '',
    ])

    # ---- CssPropertyValue message ----
    lines.extend([
        '/// A CSS property-value pair (as used in a declaration).',
        'message CssDeclaration {',
        '  CssProperty property = 1;',
        '  string value = 2;',
        '  bool important = 3;',
        '}',
        '',
    ])

    # ---- Inheritance enum ----
    lines.extend([
        '/// Whether a property inherits by default.',
        'enum Inheritance {',
        '  INHERITANCE_UNSPECIFIED = 0;',
        '  INHERITANCE_INHERITED = 1;',
        '  INHERITANCE_NOT_INHERITED = 2;',
        '  INHERITANCE_SPECIAL = 3;',
        '}',
        '',
    ])

    return '\n'.join(lines)


def generate_css_value_types_proto(catalog: dict) -> str:
    """Generate css_value_types.proto with CSS value type definitions."""
    value_types = catalog['value_types']

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.css.value_types;',
        '',
        '/// CSS Value Type Definitions — generated from W3C CSS specifications.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_css_proto.py',
        '',

        # ---- CssValueType enum ----
        '/// All CSS value types.',
        f'/// Total: {len(value_types)} value types.',
        'enum CssValueType {',
        '  CSS_VALUE_TYPE_UNSPECIFIED = 0;',
    ]

    for i, vt in enumerate(value_types):
        name = vt['name']
        # Skip values that start with - (negation) as they conflict with the positive form
        if name.startswith('-') or name.startswith('+'):
            continue
        # Skip names that aren't valid proto identifiers
        if not name.replace('-', '_').replace('.', '_')[0].isalpha():
            continue
        enum_name = to_upper_snake(name)
        lines.append(f'  {enum_name} = {i + 1}; // {name} ({vt["kind"]})')

    lines.extend([
        '}',
        '',

        # ---- ValueTypeDef message ----
        '/// Definition of a CSS value type.',
        'message ValueTypeDef {',
        '  CssValueType value_type = 1;',
        '  string name = 2;',
        '  string kind = 3; // "value" or "type"',
        '}',
        '',

        # ---- CssValue union ----
        '/// A CSS value of any type.',
        'message CssValue {',
        '  oneof value {',
        '    string keyword = 1;',
        '    string identifier = 2;',
        '    string string_value = 3;',
        '    double number_value = 4;',
        '    double length_value = 5;',
        '    string length_unit = 6;',
        '    double percentage_value = 7;',
        '    CssColor color = 8;',
        '    CssFunction function = 9;',
        '    CssUrl url = 10;',
        '  }',
        '}',
        '',

        # ---- CssColor ----
        '/// CSS color value.',
        'message CssColor {',
        '  oneof color {',
        '    string named_color = 1;',
        '    CssRgb rgb = 2;',
        '    CssHsl hsl = 3;',
        '    CssHwb hwb = 4;',
        '    CssLab lab = 5;',
        '    CssLch lch = 6;',
        '    string oklch = 7;',
        '    string oklab = 8;',
        '    string current_color = 9;',
        '    string hex = 10;',
        '  }',
        '}',
        '',
        'message CssRgb {',
        '  double r = 1;',
        '  double g = 2;',
        '  double b = 3;',
        '  optional double alpha = 4;',
        '}',
        '',
        'message CssHsl {',
        '  double h = 1;',
        '  double s = 2;',
        '  double l = 3;',
        '  optional double alpha = 4;',
        '}',
        '',
        'message CssHwb {',
        '  double h = 1;',
        '  double w = 2;',
        '  double b = 3;',
        '  optional double alpha = 4;',
        '}',
        '',
        'message CssLab {',
        '  double l = 1;',
        '  double a = 2;',
        '  double b = 3;',
        '  optional double alpha = 4;',
        '}',
        '',
        'message CssLch {',
        '  double l = 1;',
        '  double c = 2;',
        '  double h = 3;',
        '  optional double alpha = 4;',
        '}',
        '',

        # ---- CssFunction ----
        '/// CSS function call (e.g., calc(), var(), env()).',
        'message CssFunction {',
        '  string name = 1;',
        '  repeated CssValue arguments = 2;',
        '}',
        '',

        # ---- CssUrl ----
        '/// CSS url() value.',
        'message CssUrl {',
        '  string url = 1;',
        '}',
        '',

        # ---- CssKeyword enum (common keywords) ----
        '/// Common CSS keywords.',
        'enum CssKeyword {',
        '  CSS_KEYWORD_UNSPECIFIED = 0;',
        '  CSS_KEYWORD_AUTO = 1;',
        '  CSS_KEYWORD_NONE = 2;',
        '  CSS_KEYWORD_INHERIT = 3;',
        '  CSS_KEYWORD_INITIAL = 4;',
        '  CSS_KEYWORD_UNSET = 5;',
        '  CSS_KEYWORD_REVERT = 6;',
        '  CSS_KEYWORD_REVERT_LAYER = 7;',
        '  CSS_KEYWORD_CURRENT_COLOR = 8;',
        '}',
        '',
    ])

    return '\n'.join(lines)


def generate_css_at_rules_proto(catalog: dict) -> str:
    """Generate css_at_rules.proto with CSS at-rule definitions."""
    at_rules = catalog['at_rules']

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.css.at_rules;',
        '',
        'import "edgerun/v0/css/css_properties.proto";',
        '',
        '/// CSS At-Rule Definitions — generated from W3C CSS specifications.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_css_proto.py',
        '',

        # ---- CssAtRule enum ----
        '/// All CSS at-rules.',
        f'/// Total: {len(at_rules)} at-rules.',
        'enum CssAtRule {',
        '  CSS_AT_RULE_UNSPECIFIED = 0;',
    ]

    for i, rule in enumerate(at_rules):
        name = rule['name']
        enum_name = to_upper_snake(name)
        lines.append(f'  {enum_name} = {i + 1}; // @{name}')

    lines.extend([
        '}',
        '',

        # ---- CssAtRule message ----
        '/// A CSS at-rule.',
        'message CssAtRuleBlock {',
        '  CssAtRule at_rule = 1;',
        '  string prelude = 2; // The prelude (e.g., "screen and (min-width: 768px)" for @media)',
        '  repeated properties.CssDeclaration declarations = 3;',
        '  repeated CssAtRuleBlock nested_rules = 4;',
        '}',
        '',
    ])

    return '\n'.join(lines)


def main():
    if len(sys.argv) < 3:
        print("Usage: generate_css_proto.py <catalog.json> <output_dir>", file=sys.stderr)
        sys.exit(1)

    catalog = load_catalog(sys.argv[1])
    output_dir = sys.argv[2]
    os.makedirs(output_dir, exist_ok=True)

    files = {
        'css_properties.proto': generate_css_properties_proto(catalog),
        'css_value_types.proto': generate_css_value_types_proto(catalog),
        'css_at_rules.proto': generate_css_at_rules_proto(catalog),
    }

    for filename, content in files.items():
        path = os.path.join(output_dir, filename)
        with open(path, 'w') as f:
            f.write(content)
        print(f"Generated: {path}")

    print(f"\nDone. {len(files)} proto files generated.")


if __name__ == "__main__":
    main()
