#!/usr/bin/env python3
"""
Generate Rust code for the edgerun-html crate from the HTML element catalog JSON.

Output:
  crates/edgerun-html/src/gen/elements.rs    — HtmlElement enum + constructors
  crates/edgerun-html/src/gen/attributes.rs  — Attribute types + per-element attribute structs
  crates/edgerun-html/src/gen/content_model.rs — ContentModel + Category enums
  crates/edgerun-html/src/gen/dom_interfaces.rs — DOM interface trait hierarchy

Usage: python3 scripts/generate_html_crate.py scripts/html_element_catalog.json crates/edgerun-html/src/gen/
"""

import json
import os
import sys
from textwrap import dedent


def load_catalog(path: str) -> dict:
    with open(path, 'r') as f:
        return json.load(f)


def rust_ident(name: str) -> str:
    """Convert hyphenated HTML attribute name to Rust snake_case."""
    return name.replace('-', '_')


def rust_enum_variant(name: str) -> str:
    """Convert attribute name to PascalCase enum variant."""
    return ''.join(p.capitalize() for p in name.replace('-', '_').split('_'))


def generate_elements_rs(catalog: dict) -> str:
    """Generate the main HtmlElement enum."""
    elements = catalog['elements']
    lines = [
        "// Auto-generated from WHATWG HTML Living Standard",
        "// DO NOT EDIT. Regenerate with: scripts/generate_html_crate.py",
        "",
        "use crate::content_model::ContentModel;",
        "use crate::dom_interfaces::*;",
        "",
        "/// All HTML elements defined in the WHATWG HTML Living Standard.",
        f"/// Total: {len(elements)} elements.",
        "#[derive(Debug, Clone, PartialEq, Eq, Hash)]",
        "pub enum HtmlElement {",
    ]

    for el in elements:
        tag = el['tag_name']
        iface = el['dom_interface'] or 'HTMLElement'
        desc = el.get('represents', '').replace('`', '').replace('"', r'\"')
        if desc:
            lines.append(f'    /// <{tag}> — {desc}')
        else:
            lines.append(f'    /// <{tag}> element')
        lines.append(f'    {rust_ident(tag)},')
    lines.append('}')
    lines.append('')

    # tag_name() method
    lines.extend([
        'impl HtmlElement {',
        '    /// Returns the HTML tag name as a lowercase string slice.',
        '    pub fn tag_name(&self) -> &\'static str {',
        '        match self {',
    ])
    for el in elements:
        lines.append(f'            HtmlElement::{rust_ident(el["tag_name"])} => "{el["tag_name"]}",')
    lines.extend([
        '        }',
        '    }',
        '',
    ])

    # content_model() method
    lines.extend([
        '    /// Returns the content model for this element.',
        '    pub fn content_model(&self) -> ContentModel {',
        '        match self {',
    ])
    for el in elements:
        cm = classify_content_model(el['content_model'])
        lines.append(f'            HtmlElement::{rust_ident(el["tag_name"])} => {cm},')
    lines.extend([
        '        }',
        '    }',
        '',
    ])

    # dom_interface() method
    lines.extend([
        '    /// Returns the DOM interface type tag.',
        '    pub fn dom_interface(&self) -> DomInterface {',
        '        match self {',
    ])
    for el in elements:
        iface = el['dom_interface'] or 'HTMLElement'
        rust_iface = rust_ident(iface)
        lines.append(f'            HtmlElement::{rust_ident(el["tag_name"])} => DomInterface::{rust_iface},')
    lines.extend([
        '        }',
        '    }',
        '}',
        '',
    ])

    # from_tag_name
    lines.extend([
        'impl HtmlElement {',
        '    /// Parse an HTML element from a tag name string.',
        '    pub fn from_tag_name(name: &str) -> Option<Self> {',
        '        match name {',
    ])
    for el in elements:
        lines.append(f'            "{el["tag_name"]}" => Some(HtmlElement::{rust_ident(el["tag_name"])}),')
    lines.extend([
        '            _ => None,',
        '        }',
        '    }',
        '}',
        '',
    ])

    # AllElements constant
    lines.extend([
        '/// Complete list of all HTML elements.',
        'pub const ALL_ELEMENTS: &[HtmlElement] = &[',
    ])
    for el in elements:
        lines.append(f'    HtmlElement::{rust_ident(el["tag_name"])},')
    lines.extend([
        '];',
        '',
    ])

    return '\n'.join(lines)


def generate_attributes_rs(catalog: dict) -> str:
    """Generate attribute types and per-element attribute structs."""
    elements = catalog['elements']

    lines = [
        "// Auto-generated from WHATWG HTML Living Standard",
        "// DO NOT EDIT. Regenerate with: scripts/generate_html_crate.py",
        "",
        "/// Global HTML attributes common to most elements.",
        "#[derive(Debug, Clone, PartialEq, Eq, Default)]",
        "pub struct GlobalAttributes {",
        "    pub id: Option<String>,",
        "    pub class: Vec<String>,",
        "    pub style: Option<String>,",
        "    pub title: Option<String>,",
        "    pub lang: Option<String>,",
        "    pub dir: Option<TextDirection>,",
        "    pub tabindex: Option<i64>,",
        "    pub accesskey: Option<String>,",
        "    pub hidden: bool,",
        "    pub inert: bool,",
        "    pub popover: Option<PopoverState>,",
        "    pub slot: Option<String>,",
        "    pub translate: bool,",
        "}",
        "",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
        "pub enum TextDirection {",
        "    Ltr,",
        "    Rtl,",
        "    Auto,",
        "}",
        "",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
        "pub enum PopoverState {",
        "    Auto,",
        "    Manual,",
        "}",
        "",
    ]

    # Generate per-element attribute structs for elements with specific attributes
    elements_with_attrs = [e for e in elements if e['element_specific_attributes']]

    if elements_with_attrs:
        lines.append("/// Element-specific attribute structs for elements with non-global attributes.\n")

    for el in elements_with_attrs:
        tag = el['tag_name']
        var = rust_ident(tag)
        attrs = el['element_specific_attributes']

        # Description comment
        lines.append(f'/// Attributes specific to the `<{tag}>` element.')
        lines.append('#[derive(Debug, Clone, PartialEq, Eq, Default)]')
        lines.append(f'pub struct {var.title().replace("_", "")}Attributes {{')

        for attr in attrs:
            attr_name = rust_ident(attr['name'])
            attr_type = infer_attr_type(attr['name'], attr.get('description', ''))
            desc = attr.get('description', '').replace('`', '').replace('"', r'\"')
            lines.append(f'    /// {desc}')
            lines.append(f'    pub {attr_name}: {attr_type},')
        lines.append('}')
        lines.append('')

    # Enum for all attribute names
    all_attr_names = set()
    for el in elements:
        for attr in el['element_specific_attributes']:
            all_attr_names.add(attr['name'])

    if all_attr_names:
        lines.append('/// All element-specific attribute names found in the HTML spec.')
        lines.append('#[derive(Debug, Clone, PartialEq, Eq, Hash)]')
        lines.append('pub enum ElementAttributeName {')
        for name in sorted(all_attr_names):
            lines.append(f'    {rust_enum_variant(name)},')
        lines.append('}')
        lines.append('')

        # from_str
        lines.extend([
            'impl ElementAttributeName {',
            '    pub fn from_str(name: &str) -> Option<Self> {',
            '        match name {',
        ])
        for name in sorted(all_attr_names):
            lines.append(f'            "{name}" => Some(ElementAttributeName::{rust_enum_variant(name)}),')
        lines.extend([
            '            _ => None,',
            '        }',
            '    }',
            '}',
            '',
        ])

    return '\n'.join(lines)


def generate_content_model_rs(catalog: dict) -> str:
    """Generate content model types."""
    elements = catalog['elements']

    lines = [
        "// Auto-generated from WHATWG HTML Living Standard",
        "// DO NOT EDIT. Regenerate with: scripts/generate_html_crate.py",
        "",
        "/// Content model classification for HTML elements.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ContentModel {",
        "    /// Element may contain flow content (most structural elements).",
        "    Flow,",
        "    /// Element may contain phrasing content (inline text and embedded elements).",
        "    Phrasing,",
        "    /// Element accepts metadata content.",
        "    Metadata,",
        "    /// Element accepts heading content.",
        "    Heading,",
        "    /// Element accepts sectioning content.",
        "    Sectioning,",
        "    /// Element accepts embedded content (media, iframes, etc.).",
        "    Embedded,",
        "    /// Element accepts interactive content.",
        "    Interactive,",
        "    /// Element accepts palatable content (media with text alternatives).",
        "    Palatable,",
        "    /// Element accepts scripting content.",
        "    ScriptSupporting,",
        "    /// Element has no content (void/self-closing elements).",
        "    Nothing,",
        "    /// Element has a transparent content model (inherits from parent).",
        "    Transparent,",
        "    /// Element accepts only text content.",
        "    Text,",
        "    /// Element has a custom/restricted content model (hand-coded validation).",
        "    Custom,",
        "}",
        "",
        "/// Content categories defined in the HTML spec.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ContentCategory {",
        "    Metadata,",
        "    Flow,",
        "    Sectioning,",
        "    Heading,",
        "    Phrasing,",
        "    Embedded,",
        "    Interactive,",
        "    Palatable,",
        "    ScriptSupporting,",
        "}",
        "",
    ]

    # Category membership lookup
    cat_members = {}
    for el in elements:
        for cat in el.get('categories', []):
            cat_members.setdefault(cat, []).append(rust_ident(el['tag_name']))

    if cat_members:
        lines.append('impl ContentCategory {')
        lines.append('    /// Returns all element names belonging to this category.')
        lines.append('    pub fn elements(&self) -> &[&str] {')
        lines.append('        match self {')
        for cat, members in sorted(cat_members.items()):
            cat_name = rust_ident(cat.replace('-content', '').replace('-content-2', ''))
            lines.append(f'            ContentCategory::{rust_enum_variant(cat_name)} => &[')
            for m in members:
                lines.append(f'                "{m}",')
            lines.append('            ],')
        lines.extend([
            '        }',
            '    }',
            '}',
            '',
        ])

    return '\n'.join(lines)


def generate_dom_interfaces_rs(catalog: dict) -> str:
    """Generate DOM interface types."""
    elements = catalog['elements']

    # Collect unique DOM interfaces
    interfaces = {}
    for el in elements:
        iface = el['dom_interface'] or 'HTMLElement'
        if iface not in interfaces:
            interfaces[iface] = []
        interfaces[iface].append(el)

    lines = [
        "// Auto-generated from WHATWG HTML Living Standard",
        "// DO NOT EDIT. Regenerate with: scripts/generate_html_crate.py",
        "",
        "/// DOM interface type tags for HTML elements.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum DomInterface {",
    ]

    for iface_name in sorted(interfaces.keys()):
        var = rust_ident(iface_name)
        lines.append(f'    {rust_enum_variant(var)},')

    lines.extend([
        '}',
        '',
        'impl DomInterface {',
        '    pub fn name(&self) -> &\'static str {',
        '        match self {',
    ])

    for iface_name in sorted(interfaces.keys()):
        var = rust_ident(iface_name)
        lines.append(f'            DomInterface::{rust_enum_variant(var)} => "{iface_name}",')

    lines.extend([
        '        }',
        '    }',
        '}',
        '',
    ])

    # Group elements by interface
    lines.append('impl DomInterface {')
    lines.append('    /// Returns all elements that use this DOM interface.')
    lines.append('    pub fn elements(&self) -> &[&str] {')
    lines.append('        match self {')

    for iface_name in sorted(interfaces.keys()):
        var = rust_ident(iface_name)
        members = interfaces[iface_name]
        lines.append(f'            DomInterface::{rust_enum_variant(var)} => &[')
        for m in members:
            lines.append(f'                "{m["tag_name"]}",')
        lines.append('            ],')

    lines.extend([
        '        }',
        '    }',
        '}',
        '',
    ])

    return '\n'.join(lines)


def generate_mod_rs() -> str:
    """Generate the mod.rs that re-exports generated modules."""
    return dedent("""\
        // Auto-generated from WHATWG HTML Living Standard
        // DO NOT EDIT. Regenerate with: scripts/generate_html_crate.py

        //! Generated HTML element definitions from the WHATWG HTML Living Standard.

        pub mod elements;
        pub mod attributes;
        pub mod content_model;
        pub mod dom_interfaces;

        pub use elements::*;
        pub use attributes::*;
        pub use content_model::*;
        pub use dom_interfaces::*;
    """)


def generate_cargo_toml() -> str:
    """Generate Cargo.toml for the edgerun-html crate."""
    return dedent("""\
        [package]
        name = "edgerun-html"
        version = "0.1.0"
        edition = "2024"
        description = "HTML element definitions generated from the WHATWG HTML Living Standard"
        publish = false

        [dependencies]
    """)


def generate_lib_rs() -> str:
    """Generate the hand-written lib.rs that re-exports generated code."""
    return dedent("""\
        //! edgerun-html — HTML element, attribute, and DOM interface definitions.
        //!
        //! Element definitions are generated from the WHATWG HTML Living Standard.
        //! See `scripts/extract_html_spec.py` and `scripts/generate_html_crate.py`.

        #![cfg_attr(not(test), no_std)]

        pub mod gen;

        pub use gen::*;
    """)


def classify_content_model(raw: str) -> str:
    """Classify a raw content model string into a ContentModel variant."""
    if not raw:
        return 'ContentModel::Custom'

    lower = raw.lower()
    if lower in ('nothing', 'nothing.', 'void'):
        return 'ContentModel::Nothing'
    if 'transparent' in lower:
        return 'ContentModel::Transparent'
    if lower.startswith('text') or lower.startswith('text.'):
        return 'ContentModel::Text'
    if 'metadata content' in lower:
        return 'ContentModel::Metadata'
    if 'heading content' in lower or lower.startswith('heading'):
        return 'ContentModel::Heading'
    if 'sectioning content' in lower:
        return 'ContentModel::Sectioning'
    if 'flow content' in lower:
        return 'ContentModel::Flow'
    if 'phrasing content' in lower or 'text that is not' in lower:
        return 'ContentModel::Phrasing'
    if 'embedded content' in lower:
        return 'ContentModel::Embedded'
    if 'interactive content' in lower:
        return 'ContentModel::Interactive'
    if 'palatable content' in lower:
        return 'ContentModel::Palatable'
    if 'script-supporting' in lower:
        return 'ContentModel::ScriptSupporting'
    if 'a `head` element followed by a `body`':
        return 'ContentModel::Custom'  # special case: html element

    # If it describes specific child elements, it's custom
    if 'element' in lower or 'elements' in lower:
        return 'ContentModel::Custom'

    return 'ContentModel::Custom'


def infer_attr_type(name: str, description: str) -> str:
    """Infer a Rust type for an HTML attribute."""
    desc_lower = description.lower()

    # Boolean attributes
    if name in ('disabled', 'readonly', 'required', 'multiple', 'hidden',
                'reversed', 'open', 'selected', 'checked', 'autoplay',
                'controls', 'loop', 'muted', 'default', 'nomodule',
                'novalidate', 'formnovalidate', 'allowfullscreen',
                'allowpaymentrequest', 'playsinline', 'blocking'):
        return 'bool'

    # Numeric
    if name in ('tabindex', 'maxlength', 'minlength', 'size', 'span',
                'colspan', 'rowspan', 'start', 'value', 'min', 'max',
                'step', 'width', 'height', 'low', 'high', 'optimum'):
        if name in ('min', 'max', 'value', 'low', 'high', 'optimum', 'step'):
            return 'Option<f64>'
        return 'Option<u32>'

    # URL/URI
    if name in ('href', 'src', 'action', 'formaction', 'cite', 'data',
                'poster', 'icon', 'manifest', 'profile'):
        return 'Option<String>'

    # Space-separated token lists
    if name in ('class', 'sizes', 'accept', 'accept-charset', 'enctype',
                'ping', 'rel', 'rev', 'noreferrer', 'noopener', 'referrerpolicy'):
        return 'Vec<String>'

    # Enumerated
    if name == 'type':
        return 'Option<String>'
    if name in ('method', 'formmethod'):
        return 'Option<String>'
    if name in ('direction', 'dir'):
        return 'Option<String>'
    if name in ('wrap'):
        return 'Option<String>'
    if name in ('kind', 'loading', 'decoding', 'fetchpriority', 'as',
                'shape', 'crossorigin', 'integrity', 'sandbox',
                'autocomplete', 'inputmode', 'enterkeyhint', 'popover'):
        return 'Option<String>'

    # Media queries
    if name == 'media':
        return 'Option<String>'

    # String (default)
    if name in ('name', 'content', 'id', 'class', 'style', 'title',
                'lang', 'dir', 'slot', 'alt', 'download', 'hreflang',
                'charset', 'color', 'placeholder', 'pattern', 'value',
                'role', 'datetime', 'aria-*'):
        return 'Option<String>'

    return 'Option<String>'


def main():
    if len(sys.argv) < 3:
        print("Usage: generate_html_crate.py <catalog.json> <output_dir>", file=sys.stderr)
        sys.exit(1)

    catalog_path = sys.argv[1]
    output_dir = sys.argv[2]
    gen_dir = os.path.join(output_dir, 'gen')

    catalog = load_catalog(catalog_path)

    # Create output directories
    os.makedirs(gen_dir, exist_ok=True)
    os.makedirs(output_dir, exist_ok=True)

    # Generate files
    files = {
        'gen/mod.rs': generate_mod_rs(),
        'gen/elements.rs': generate_elements_rs(catalog),
        'gen/attributes.rs': generate_attributes_rs(catalog),
        'gen/content_model.rs': generate_content_model_rs(catalog),
        'gen/dom_interfaces.rs': generate_dom_interfaces_rs(catalog),
        'Cargo.toml': generate_cargo_toml(),
        'src/lib.rs': generate_lib_rs(),
    }

    for rel_path, content in files.items():
        full_path = os.path.join(output_dir, rel_path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)
        with open(full_path, 'w') as f:
            f.write(content)
        print(f"Generated: {rel_path}")

    print(f"\nDone. Generated {len(files)} files to {output_dir}/")


if __name__ == "__main__":
    main()
