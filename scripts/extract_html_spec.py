#!/usr/bin/env python3
"""
Extract HTML element definitions from the WHATWG HTML Living Standard (markdown).
Outputs a JSON catalog suitable for Rust code generation.

Usage: python3 scripts/extract_html_spec.py docs/html_spec.md > scripts/html_element_catalog.json
"""

import json
import re
import sys


def extract_elements(markdown_text: str) -> list[dict]:
    """Parse the HTML spec markdown and extract element definitions."""
    elements = []

    # Pattern to find element definition headers like:
    # #### <span class="secno">4.1.1</span> The <span class="dfn" dfn-type="element">`html`</span> element
    element_header_re = re.compile(
        r'^####\s+.*?'
        r'<span[^>]*?dfn-type="element">`(\w+)`</span>',
        re.MULTILINE
    )

    # Find all element positions
    matches = list(element_header_re.finditer(markdown_text))

    for i, match in enumerate(matches):
        tag_name = match.group(1)
        start = match.start()
        end = matches[i + 1].start() if i + 1 < len(matches) else len(markdown_text)
        block = markdown_text[start:end]

        element = {
            "tag_name": tag_name,
            "categories": [],
            "contexts": "",
            "content_model": "",
            "tag_omission": "",
            "has_global_attributes": False,
            "element_specific_attributes": [],
            "dom_interface": "",
            "idl": "",
            "represents": "",
        }

        # Extract categories
        cat_m = re.search(
            r'<a href="#concept-element-categories".*?</a>:\s*\n(.*?)(?=\n\n|\n<a href="#concept-element)',
            block, re.DOTALL
        )
        if cat_m:
            raw = cat_m.group(1).strip()
            if raw.lower() != "none.":
                # Extract category links
                cats = re.findall(r'<a href="#([^"]+?)-content[^"]*"[^>]*>([^<]+)</a>', raw)
                element["categories"] = [c[0] for c in cats] if cats else [raw]
            else:
                element["categories"] = []

        # Extract contexts
        ctx_m = re.search(
            r'<a href="#concept-element-contexts".*?</a>:\s*\n(.*?)(?=\n\n|\n<a href="#concept-element-content-model)',
            block, re.DOTALL
        )
        if ctx_m:
            element["contexts"] = _strip_html(ctx_m.group(1).strip())

        # Extract content model
        cm_m = re.search(
            r'<a href="#concept-element-content-model".*?</a>:\s*\n(.*?)(?=\n\n|\n<a href="#concept-element-tag-omission)',
            block, re.DOTALL
        )
        if cm_m:
            element["content_model"] = _strip_html(cm_m.group(1).strip())

        # Extract tag omission
        to_m = re.search(
            r'<a href="#concept-element-tag-omission".*?</a>:\s*\n(.*?)(?=\n\n|\n<a href="#concept-element-attributes)',
            block, re.DOTALL
        )
        if to_m:
            element["tag_omission"] = _strip_html(to_m.group(1).strip())

        # Extract content attributes section
        attrs_block = _extract_attrs_block(block)
        if attrs_block:
            if "global-attributes" in attrs_block or "Global\nattributes" in attrs_block or "Global attributes" in attrs_block:
                element["has_global_attributes"] = True

            # Extract element-specific attributes: [`attrname`](...) — description
            attr_entries = re.findall(
                r'\[`(\w+)`\]\([^)]*\)\s*—\s*(.+?)(?=\n\n|\n\[`|\n<a href="#concept-element-accessibility|\n<a href="#concept-element-dom)',
                attrs_block, re.DOTALL
            )
            for attr_name, attr_desc in attr_entries:
                desc = _strip_html(attr_desc.strip())
                # Collapse whitespace
                desc = re.sub(r'\s+', ' ', desc)
                element["element_specific_attributes"].append({
                    "name": attr_name,
                    "description": desc,
                })

        # Extract DOM interface (IDL block)
        idl_m = re.search(
            r'<a href="#concept-element-dom".*?</a>:\s*\n``` idl\n(.*?)```',
            block, re.DOTALL
        )
        if idl_m:
            element["idl"] = idl_m.group(1).strip()
            # Extract interface name
            iface_m = re.match(r'(?:\[Exposed=Window\]\s*\n)?interface\s+(\w+)', element["idl"])
            if iface_m:
                element["dom_interface"] = iface_m.group(1)

        # Extract what the element "represents" (short description)
        rep_m = re.search(
            r'`' + re.escape(tag_name) + r'`.*?<a href="#represents"[^>]*>represents</a>\s+(.+?)(?:\.|\n\n)',
            block, re.DOTALL
        )
        if rep_m:
            element["represents"] = _strip_html(rep_m.group(1).strip())

        elements.append(element)

    return elements


def _extract_attrs_block(block: str) -> str:
    """Extract the content attributes section from an element block."""
    # Find the "Content attributes" heading
    m = re.search(
        r'concept-element-attributes.*?Content attributes[</a>:]*\s*\n(.*?)(?='
        r'<a href="#concept-element-accessibility-considerations"|'
        r'<a href="#concept-element-dom"|'
        r'^####)',
        block, re.DOTALL
    )
    if m:
        return m.group(1)
    return ""


def _strip_html(text: str) -> str:
    """Remove HTML tags from text, collapsing whitespace."""
    # Remove HTML tags
    text = re.sub(r'<[^>]+>', '', text)
    # Decode common HTML entities
    text = text.replace('&amp;', '&')
    text = text.replace('&lt;', '<')
    text = text.replace('&gt;', '>')
    text = text.replace('&#39;', "'")
    text = text.replace('&quot;', '"')
    text = text.replace('&nbsp;', ' ')
    # Collapse whitespace
    text = re.sub(r'\s+', ' ', text).strip()
    return text


def main():
    if len(sys.argv) < 2:
        print("Usage: extract_html_spec.py <path-to-html-spec.md>", file=sys.stderr)
        sys.exit(1)

    with open(sys.argv[1], 'r', encoding='utf-8') as f:
        markdown_text = f.read()

    elements = extract_elements(markdown_text)

    catalog = {
        "spec": "WHATWG HTML Living Standard",
        "spec_date": "2026-04-07",
        "total_elements": len(elements),
        "elements": elements,
    }

    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    main()
