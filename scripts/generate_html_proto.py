#!/usr/bin/env python3
"""
Generate protobuf definitions for HTML elements from the catalog JSON.
Outputs proto files under proto/edgerun/v0/html/

Usage: python3 scripts/generate_html_proto.py scripts/html_element_catalog.json proto/edgerun/v0/html/
"""

import json
import os
import sys
from collections import defaultdict


def load_catalog(path: str) -> dict:
    with open(path, 'r') as f:
        return json.load(f)


def proto_field_number(idx: int) -> int:
    return idx + 1


def generate_html_elements_proto(catalog: dict) -> str:
    """Generate the main html_elements.proto with element enum and attribute messages."""
    elements = catalog['elements']

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.html.elements;',
        '',
        '/// HTML Element Definitions — generated from the WHATWG HTML Living Standard.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_html_proto.py',
        '',

        # ---- HtmlElement enum ----
        '/// All HTML elements defined in the WHATWG HTML Living Standard.',
        f'/// Total: {len(elements)} elements.',
        'enum HtmlElement {',
        '  HTML_ELEMENT_UNSPECIFIED = 0;',
    ]

    for i, el in enumerate(elements):
        tag = el['tag_name']
        enum_name = to_upper_snake(tag)
        desc = el.get('represents', '').replace('`', '').replace('"', '\\"').replace('\n', ' ')
        if desc and len(desc) < 120:
            lines.append(f'  // <{tag}> — {desc}')
        lines.append(f'  {enum_name} = {i + 1};')

    lines.extend([
        '}',
        '',
    ])

    # ---- ContentModel enum ----
    lines.extend([
        '/// Content model classification for HTML elements.',
        'enum ContentModel {',
        '  CONTENT_MODEL_UNSPECIFIED = 0;',
        '  CONTENT_MODEL_FLOW = 1;',
        '  CONTENT_MODEL_PHRASING = 2;',
        '  CONTENT_MODEL_METADATA = 3;',
        '  CONTENT_MODEL_HEADING = 4;',
        '  CONTENT_MODEL_SECTIONING = 5;',
        '  CONTENT_MODEL_EMBEDDED = 6;',
        '  CONTENT_MODEL_INTERACTIVE = 7;',
        '  CONTENT_MODEL_PALATABLE = 8;',
        '  CONTENT_MODEL_SCRIPT_SUPPORTING = 9;',
        '  CONTENT_MODEL_NOTHING = 10;',
        '  CONTENT_MODEL_TRANSPARENT = 11;',
        '  CONTENT_MODEL_TEXT = 12;',
        '  CONTENT_MODEL_CUSTOM = 13;',
        '}',
        '',

        # ---- ContentCategory enum ----
        '/// Content categories used in element classification.',
        'enum ContentCategory {',
        '  CONTENT_CATEGORY_UNSPECIFIED = 0;',
        '  CONTENT_CATEGORY_METADATA = 1;',
        '  CONTENT_CATEGORY_FLOW = 2;',
        '  CONTENT_CATEGORY_SECTIONING = 3;',
        '  CONTENT_CATEGORY_HEADING = 4;',
        '  CONTENT_CATEGORY_PHRASING = 5;',
        '  CONTENT_CATEGORY_EMBEDDED = 6;',
        '  CONTENT_CATEGORY_INTERACTIVE = 7;',
        '  CONTENT_CATEGORY_PALATABLE = 8;',
        '  CONTENT_CATEGORY_SCRIPT_SUPPORTING = 9;',
        '}',
        '',
    ])

    # ---- ElementDefinition message ----
    lines.extend([
        '/// Complete definition of an HTML element.',
        'message ElementDefinition {',
        '  HtmlElement tag = 1;',
        '  string tag_name = 2;',
        '  ContentModel content_model = 3;',
        '  repeated ContentCategory categories = 4;',
        '  string content_model_description = 5;',
        '  string tag_omission = 6;',
        '  bool has_global_attributes = 7;',
        '  repeated ElementAttribute element_specific_attributes = 8;',
        '  string dom_interface = 9;',
        '  string idl = 10;',
        '}',
        '',

        '/// An HTML element-specific attribute.',
        'message ElementAttribute {',
        '  string name = 1;',
        '  string description = 2;',
        '  AttributeType attr_type = 3;',
        '}',
        '',

        '/// Inferred Rust type for an attribute.',
        'enum AttributeType {',
        '  ATTRIBUTE_TYPE_UNSPECIFIED = 0;',
        '  ATTRIBUTE_TYPE_BOOL = 1;',
        '  ATTRIBUTE_TYPE_STRING = 2;',
        '  ATTRIBUTE_TYPE_STRING_LIST = 3;',
        '  ATTRIBUTE_TYPE_UINT32 = 4;',
        '  ATTRIBUTE_TYPE_FLOAT64 = 5;',
        '  ATTRIBUTE_TYPE_ENUM = 6;',
        '}',
        '',
    ])

    # ---- ElementCatalog — registry of all elements ----
    lines.extend([
        '/// Catalog of all HTML element definitions.',
        'message ElementCatalog {',
        '  string spec = 1;',
        '  string spec_date = 2;',
        '  repeated ElementDefinition elements = 3;',
        '}',
        '',
    ])

    return '\n'.join(lines)


def generate_html_attributes_proto(catalog: dict) -> str:
    """Generate per-element attribute messages for elements with specific attrs."""
    elements = catalog['elements']
    elements_with_attrs = [e for e in elements if e['element_specific_attributes']]

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.html.attributes;',
        '',
        '/// HTML Element-Specific Attribute Messages.',
        '/// Each message represents the attribute set for one HTML element.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_html_proto.py',
        '',

        # ---- GlobalAttributes ----
        '/// Global HTML attributes common to most elements.',
        'message GlobalAttributes {',
        '  string id = 1;',
        '  repeated string classes = 2;',
        '  string style = 3;',
        '  string title = 4;',
        '  string lang = 5;',
        '  TextDirection dir = 6;',
        '  int64 tabindex = 7;',
        '  string accesskey = 8;',
        '  bool hidden = 9;',
        '  bool inert = 10;',
        '  PopoverState popover = 11;',
        '  string slot = 12;',
        '  bool translate = 13;',
        '}',
        '',
        'enum TextDirection {',
        '  TEXT_DIRECTION_UNSPECIFIED = 0;',
        '  TEXT_DIRECTION_LTR = 1;',
        '  TEXT_DIRECTION_RTL = 2;',
        '  TEXT_DIRECTION_AUTO = 3;',
        '}',
        '',
        'enum PopoverState {',
        '  POPOVER_STATE_UNSPECIFIED = 0;',
        '  POPOVER_STATE_AUTO = 1;',
        '  POPOVER_STATE_MANUAL = 2;',
        '}',
        '',
    ]

    for el in elements_with_attrs:
        tag = el['tag_name']
        msg_name = to_pascal_case(tag) + 'Attributes'
        desc = el.get('represents', '').replace('`', '').replace('"', '\\"')

        lines.append(f'/// Attributes for the <{tag}> element.')
        if desc and len(desc) < 100:
            lines.append(f'/// {desc}')
        lines.append(f'message {msg_name} {{')

        # Reference to element
        lines.append(f'  // Element: <{tag}>')
        lines.append('')

        for i, attr in enumerate(el['element_specific_attributes']):
            attr_name = to_snake_case(attr['name'])
            attr_type = infer_proto_type(attr['name'], attr.get('description', ''))
            desc = attr.get('description', '').replace('`', '').replace('"', '\\"').replace('\n', ' ')
            if desc and len(desc) < 120:
                lines.append(f'  // {desc}')
            lines.append(f'  {attr_type} {attr_name} = {i + 1};')

        lines.extend(['}', ''])

    return '\n'.join(lines)


def generate_html_dom_interfaces_proto(catalog: dict) -> dict:
    """Generate DOM interface messages grouped by interface."""
    elements = catalog['elements']

    # Group elements by DOM interface
    interfaces = defaultdict(list)
    for el in elements:
        iface = el['dom_interface'] or 'HTMLElement'
        interfaces[iface].append(el)

    lines = [
        'syntax = "proto3";',
        '',
        'package edgerun.v0.html.dom_interfaces;',
        '',
        '/// HTML DOM Interface Definitions.',
        '/// DO NOT EDIT. Regenerate with: scripts/generate_html_proto.py',
        '',
    ]

    for iface_name in sorted(interfaces.keys()):
        members = interfaces[iface_name]
        lines.append(f'/// {iface_name} — used by {len(members)} element(s): {", ".join(f"<{m["tag_name"]}>" for m in members)}')
        lines.append(f'message {iface_name} {{')

        # Extract IDL attributes
        for el in members:
            if el.get('idl'):
                idl_attrs = parse_idl_attributes(el['idl'])
                if idl_attrs:
                    lines.append(f'  // === {el["tag_name"]} ===')
                    for attr_name, attr_type in idl_attrs:
                        proto_type = idl_to_proto_type(attr_type)
                        if proto_type:
                            lines.append(f'  {proto_type} {to_snake_case(attr_name)} = 1; // {el["tag_name"]}')
                break  # just show first one as example

        lines.extend(['}', ''])

    # Enum of all interfaces
    lines.append('/// All DOM interface types.')
    lines.append('enum DomInterface {')
    lines.append('  DOM_INTERFACE_UNSPECIFIED = 0;')
    for iface_name in sorted(interfaces.keys()):
        lines.append(f'  DOM_INTERFACE_{to_upper_snake(iface_name)} = {list(sorted(interfaces.keys())).index(iface_name) + 1};')
    lines.extend(['}', ''])

    return '\n'.join(lines)


def parse_idl_attributes(idl: str) -> list[tuple[str, str]]:
    """Extract attribute declarations from IDL."""
    attrs = []
    for line in idl.split('\n'):
        line = line.strip()
        # Match: [CEReactions, Reflect] attribute DOMString foo;
        m = __import__('re').search(r'attribute\s+(\w+(?:<[^>]+>)?)\s+(\w+)', line)
        if m and 'constructor' not in line.lower():
            attrs.append((m.group(2), m.group(1)))
    return attrs


def idl_to_proto_type(idl_type: str) -> str:
    """Map WebIDL type to proto type."""
    mapping = {
        'DOMString': 'string',
        'USVString': 'string',
        'ByteString': 'string',
        'boolean': 'bool',
        'unsigned short': 'uint32',
        'unsigned long': 'uint32',
        'unsigned long long': 'uint64',
        'long': 'int32',
        'long long': 'int64',
        'double': 'double',
        'float': 'float',
    }
    return mapping.get(idl_type, None)


def infer_proto_type(name: str, description: str) -> str:
    """Infer a proto field type for an HTML attribute."""
    # Boolean
    if name in ('disabled', 'readonly', 'required', 'multiple', 'hidden',
                'reversed', 'open', 'selected', 'checked', 'autoplay',
                'controls', 'loop', 'muted', 'default', 'nomodule',
                'novalidate', 'formnovalidate', 'allowfullscreen',
                'allowpaymentrequest', 'playsinline', 'blocking'):
        return 'bool'

    # Numeric
    if name in ('tabindex', 'maxlength', 'minlength', 'size', 'span',
                'colspan', 'rowspan', 'start'):
        return 'uint32'

    if name in ('min', 'max', 'value', 'low', 'high', 'optimum', 'step'):
        return 'double'

    if name in ('width', 'height'):
        return 'uint32'

    # String lists
    if name in ('ping', 'sizes', 'rel', 'accept'):
        return 'repeated string'

    # Default: string
    return 'string'


def to_upper_snake(s: str) -> str:
    return s.replace('-', '_').upper()


def to_snake_case(s: str) -> str:
    return s.replace('-', '_')


def to_pascal_case(s: str) -> str:
    return ''.join(p.capitalize() for p in s.replace('-', '_').split('_'))


def main():
    if len(sys.argv) < 3:
        print("Usage: generate_html_proto.py <catalog.json> <output_dir>", file=sys.stderr)
        sys.exit(1)

    catalog = load_catalog(sys.argv[1])
    output_dir = sys.argv[2]
    os.makedirs(output_dir, exist_ok=True)

    files = {
        'html_elements.proto': generate_html_elements_proto(catalog),
        'html_attributes.proto': generate_html_attributes_proto(catalog),
        'html_dom_interfaces.proto': generate_html_dom_interfaces_proto(catalog),
    }

    for filename, content in files.items():
        path = os.path.join(output_dir, filename)
        with open(path, 'w') as f:
            f.write(content)
        print(f"Generated: {path}")

    print(f"\nDone. {len(files)} proto files generated.")


if __name__ == "__main__":
    main()
