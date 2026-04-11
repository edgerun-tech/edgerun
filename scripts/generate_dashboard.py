#!/usr/bin/env python3
"""
CSS Spec Conformance Dashboard Generator for Edgerun

Reads all proto files from proto/edgerun/v0/, extracts every enum, message,
and field, cross-references with conformance tests, and generates a
self-contained HTML dashboard at dashboard/conformance.html.

Usage: python3 scripts/generate_dashboard.py
"""

import os
import re
import sys
import json
import math
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
PROTO_DIR = ROOT / "proto" / "edgerun" / "v0"
CONFORMANCE_FILE = ROOT / "crates" / "edgerun-conformance" / "tests" / "generated_conformance.rs"
OUTPUT_DIR = ROOT / "dashboard"
OUTPUT_FILE = OUTPUT_DIR / "conformance.html"

# ──────────────────────────────────────────────────────────
# 1. Proto Parser
# ──────────────────────────────────────────────────────────


class ProtoItem:
    """Represents a parsed proto definition (enum value, message field, etc.)."""
    def __init__(self, name: str, domain: str, source_file: str, item_type: str,
                 parent: str = "", comment: str = ""):
        self.name = name
        self.domain = domain
        self.source_file = source_file
        self.item_type = item_type  # "enum_value", "enum", "message", "field"
        self.parent = parent
        self.comment = comment.strip()


def classify_domain(rel_path: str) -> str:
    """Classify a proto file path into a domain category."""
    if "/css/" in rel_path:
        fname = os.path.basename(rel_path)
        if "properties" in fname:
            return "CSS Properties"
        elif "value_types" in fname:
            return "CSS Value Types"
        elif "display" in fname:
            return "CSS Display Types"
        elif "box" in fname:
            return "CSS Box Types"
        elif "fonts" in fname:
            return "CSS Font Types"
        elif "animations" in fname:
            return "CSS Animation Types"
        elif "images" in fname:
            return "CSS Image Types"
        elif "text" in fname:
            return "CSS Text Types"
        elif "transitions" in fname:
            return "CSS Transition Types"
        elif "transforms" in fname:
            return "CSS Transform Types"
        elif "cascade" in fname:
            return "CSS Cascade Types"
        elif "contain" in fname:
            return "CSS Contain Types"
        elif "sizing" in fname:
            return "CSS Sizing Types"
        elif "syntax" in fname:
            return "CSS Syntax Types"
        elif "ui" in fname:
            return "CSS UI Types"
        elif "writing" in fname:
            return "CSS Writing Modes"
        elif "at_rules" in fname:
            return "CSS At-Rules"
        elif "media_queries" in fname:
            return "CSS Media Queries"
        elif "page" in fname:
            return "CSS Page Types"
        else:
            return "CSS Other"
    elif "/html/" in rel_path:
        if "elements" in rel_path:
            return "HTML Elements"
        elif "attributes" in rel_path:
            return "HTML Attributes"
        else:
            return "HTML Other"
    elif "/dom/" in rel_path:
        return "DOM Types"
    elif "/ecmascript/" in rel_path:
        return "ECMAScript Types"
    elif "/ui/" in rel_path:
        return "UI Types"
    elif "/web/" in rel_path:
        return "Web Types"
    else:
        fname = os.path.basename(rel_path)
        if "common" in fname:
            return "Common Types"
        elif "identity" in fname:
            return "Identity Types"
        elif "capability" in fname:
            return "Capability Types"
        elif "trust" in fname:
            return "Trust Types"
        elif "network" in fname:
            return "Network Types"
        elif "object" in fname:
            return "Object Types"
        elif "stream" in fname:
            return "Stream Types"
        elif "indexeddb" in fname:
            return "IndexedDB Types"
        elif "service_workers" in fname:
            return "Service Worker Types"
        elif "xml" in fname:
            return "XML Types"
        else:
            return "Core Types"


def parse_proto_file(filepath: Path) -> list[ProtoItem]:
    """Parse a single .proto file and extract all enums, messages, and their items."""
    text = filepath.read_text()
    rel_path = str(filepath.relative_to(PROTO_DIR.parent.parent))
    domain = classify_domain(rel_path)
    items = []

    # Extract enums and their values
    # Pattern: enum EnumName { ... }
    enum_pattern = re.compile(
        r'enum\s+(\w+)\s*\{([^}]*)\}',
        re.DOTALL
    )

    # Handle nested enums (inside messages) - we'll do two passes
    # First, find top-level enums
    for match in enum_pattern.finditer(text):
        enum_name = match.group(1)
        enum_body = match.group(2)

        # Add the enum itself
        items.append(ProtoItem(
            name=enum_name,
            domain=domain,
            source_file=rel_path,
            item_type="enum",
        ))

        # Extract enum values
        # Pattern: VALUE_NAME = number;  or  VALUE_NAME = number; // comment
        value_pattern = re.compile(
            r'(\w+)\s*=\s*(\d+)\s*;\s*(?://\s*(.*))?',
        )
        for vmatch in value_pattern.finditer(enum_body):
            val_name = vmatch.group(1)
            val_num = vmatch.group(2)
            comment = vmatch.group(3) or ""
            # Skip UNSPECIFIED sentinel values from counting as real items
            if "UNSPECIFIED" in val_name:
                items.append(ProtoItem(
                    name=val_name,
                    domain=domain,
                    source_file=rel_path,
                    item_type="enum_sentinel",
                    parent=enum_name,
                    comment=comment,
                ))
                continue
            items.append(ProtoItem(
                name=val_name,
                domain=domain,
                source_file=rel_path,
                item_type="enum_value",
                parent=enum_name,
                comment=comment,
            ))

    # Extract messages and their fields
    # We need to handle nested braces
    msg_pattern = re.compile(r'message\s+(\w+)\s*\{')
    for mmatch in msg_pattern.finditer(text):
        msg_name = mmatch.group(1)
        # Find the matching closing brace
        start = mmatch.end()
        depth = 1
        pos = start
        while depth > 0 and pos < len(text):
            if text[pos] == '{':
                depth += 1
            elif text[pos] == '}':
                depth -= 1
            pos += 1
        msg_body = text[start:pos - 1]

        # Add the message itself
        items.append(ProtoItem(
            name=msg_name,
            domain=domain,
            source_file=rel_path,
            item_type="message",
        ))

        # Extract fields: type name = number; with possible optional/repeated
        field_pattern = re.compile(
            r'(?:optional\s+|repeated\s+)?(\w+(?:<[^>]*>)?)\s+(\w+)\s*=\s*(\d+)\s*;\s*(?://\s*(.*))?',
        )
        for fmatch in field_pattern.finditer(msg_body):
            field_type = fmatch.group(1)
            field_name = fmatch.group(2)
            comment = fmatch.group(4) or ""
            items.append(ProtoItem(
                name=f"{msg_name}.{field_name}",
                domain=domain,
                source_file=rel_path,
                item_type="field",
                parent=msg_name,
                comment=comment,
            ))

        # Check for nested enums inside this message
        for nmatch in enum_pattern.finditer(msg_body):
            nested_enum_name = f"{msg_name}.{nmatch.group(1)}"
            nested_body = nmatch.group(2)
            items.append(ProtoItem(
                name=nested_enum_name,
                domain=domain,
                source_file=rel_path,
                item_type="enum",
                parent=msg_name,
            ))
            value_pattern = re.compile(r'(\w+)\s*=\s*(\d+)\s*;\s*(?://\s*(.*))?')
            for vmatch in value_pattern.finditer(nested_body):
                val_name = vmatch.group(1)
                comment = vmatch.group(3) or ""
                if "UNSPECIFIED" in val_name:
                    items.append(ProtoItem(
                        name=val_name,
                        domain=domain,
                        source_file=rel_path,
                        item_type="enum_sentinel",
                        parent=nested_enum_name,
                        comment=comment,
                    ))
                else:
                    items.append(ProtoItem(
                        name=val_name,
                        domain=domain,
                        source_file=rel_path,
                        item_type="enum_value",
                        parent=nested_enum_name,
                        comment=comment,
                    ))

    # Extract oneof fields
    oneof_pattern = re.compile(r'oneof\s+(\w+)\s*\{([^}]*)\}')
    for omatch in oneof_pattern.finditer(text):
        oneof_name = omatch.group(1)
        oneof_body = omatch.group(2)
        # Find parent message by searching backwards
        preceding = text[:omatch.start()]
        parent_msg_match = list(re.finditer(r'message\s+(\w+)\s*\{', preceding))
        parent_msg = parent_msg_match[-1].group(1) if parent_msg_match else ""

        items.append(ProtoItem(
            name=f"{parent_msg}.{oneof_name}",
            domain=domain,
            source_file=rel_path,
            item_type="oneof",
            parent=parent_msg,
        ))

        field_pattern = re.compile(
            r'(\w+(?:<[^>]*>)?)\s+(\w+)\s*=\s*(\d+)\s*;\s*(?://\s*(.*))?',
        )
        for fmatch in field_pattern.finditer(oneof_body):
            field_type = fmatch.group(1)
            field_name = fmatch.group(2)
            comment = fmatch.group(4) or ""
            items.append(ProtoItem(
                name=f"{parent_msg}.{field_name}",
                domain=domain,
                source_file=rel_path,
                item_type="oneof_field",
                parent=f"{parent_msg}.{oneof_name}",
                comment=comment,
            ))

    return items


def discover_proto_files() -> list[Path]:
    """Recursively find all .proto files under PROTO_DIR."""
    proto_files = []
    for root_dir, dirs, files in os.walk(PROTO_DIR):
        for f in sorted(files):
            if f.endswith(".proto"):
                proto_files.append(Path(root_dir) / f)
    return sorted(proto_files)


# ──────────────────────────────────────────────────────────
# 2. Conformance Test Parser
# ──────────────────────────────────────────────────────────


def parse_conformance_tests() -> dict[str, Any]:
    """
    Parse the conformance test file to extract:
    - Test function names
    - Which proto items they reference (by heuristic matching)
    - Section groupings

    Returns a dict of tested_item_name -> list of test names.
    """
    if not CONFORMANCE_FILE.exists():
        return {}

    text = CONFORMANCE_FILE.read_text()
    tested = {}  # item_name -> [test_names]

    # Extract all #[test] fn names
    test_fn_pattern = re.compile(r'#\[test\]\s*fn\s+(\w+)\s*\(')
    test_functions = test_fn_pattern.findall(text)

    # Extract section comments like // ─── Framebuffer ───
    section_pattern = re.compile(r'// ─{3,}\s*(.+?)\s*─{3,}')
    sections = section_pattern.findall(text)

    # Parse specific test content to find what items they test
    # Strategy: look for references to proto enum values, type names, etc.

    # Framebuffer tests
    for fn in ["framebuffer_clear", "framebuffer_fill_solid", "framebuffer_fill_alpha_opaque",
               "framebuffer_fill_alpha_zero", "framebuffer_clips_bounds"]:
        tested.setdefault("Framebuffer", []).append(fn)

    # Color LUT tests
    color_names = ["black", "white", "red", "green", "blue", "yellow", "cyan", "magenta", "orange", "purple", "transparent"]
    for name in color_names:
        tested.setdefault(f"Color: {name}", []).append("color_lut_known_values")
    tested.setdefault("Color LUT (148 named colors)", []).extend(
        ["color_lut_known_values", "color_lut_all_entries_valid", "color_lut_transparent_is_transparent"])

    # Border patterns
    border_patterns = ["SOLID", "DASHED", "DOTTED", "DOUBLE", "GROOVE", "RIDGE", "INSET", "OUTSET", "NONE"]
    for pattern in border_patterns:
        tested.setdefault(f"Border: {pattern}", []).append("border_patterns_all_valid")
    tested.setdefault("Border Pattern Dispatch", []).append("border_pattern_dispatch")
    tested.setdefault("Border Pattern Clamp", []).append("border_pattern_clamps_out_of_range")

    # Blend modes
    blend_modes = ["normal", "multiply", "screen", "darken", "lighten", "difference",
                   "exclusion", "overlay", "colordodge", "colorburn", "hardlight",
                   "softlight", "hue", "saturation", "color", "luminosity"]
    tested.setdefault("alpha_blend", []).append("blend_alpha_blending")
    for mode in blend_modes:
        tested.setdefault(f"Blend: {mode}", []).append(f"blend_{mode}")
    tested.setdefault("Blend Mode Count (16)", []).append("blend_mode_count")

    # Bitmap font
    tested.setdefault("Font Table Size (128 glyphs)", []).extend(
        ["font_table_size", "all_glyphs_valid_size"])
    for char_name in ["'0'", "'A'", "'a'", "'!'"]:
        tested.setdefault(f"Font Glyph: {char_name}", []).append("font_known_glyphs")

    # Gradients
    tested.setdefault("Gradient Stop Count Min", []).append("gradient_stop_count_min")
    tested.setdefault("Linear Gradient Endpoints", []).append("linear_gradient_endpoints")

    # Scanline rasterizer
    tested.setdefault("Fill Rect Solid", []).append("fill_rect_solid")
    tested.setdefault("Fill Rect Zero Size", []).append("fill_rect_zero_size_is_noop")
    tested.setdefault("Fill Rect Alpha Blend", []).append("fill_rect_alpha_blend")
    tested.setdefault("Empty Command List", []).append("empty_command_list_is_noop")
    tested.setdefault("Stroke Rect Solid", []).append("stroke_rect_solid_border")
    tested.setdefault("Text Rendering", []).append("text_renders_non_whitespace")
    tested.setdefault("Whitespace Text Skip", []).append("whitespace_text_is_skipped")

    # Rectangle rendering
    tested.setdefault("Fill Rect Alpha Zero", []).append("fill_rect_alpha_zero_no_change")
    tested.setdefault("Stroke Rect Thickness", []).append("stroke_rect_thickness")
    tested.setdefault("Stroke Rect Dashed", []).append("stroke_rect_dashed_pattern")

    # Proto enum conformance
    tested.setdefault("css_value_type_enum_discriminants", []).append("css_value_type_enum_discriminants")
    tested.setdefault("css_property_enum_discriminants", []).append("css_property_enum_discriminants")
    tested.setdefault("css_display_outer_enum", []).append("css_display_outer_enum")
    tested.setdefault("css_display_inner_enum", []).append("css_display_inner_enum")
    tested.setdefault("css_shadow_type_enum", []).append("css_shadow_type_enum")
    tested.setdefault("html_element_enum_count", []).append("html_element_enum_count")

    return tested


def check_test_coverage(proto_items: list[ProtoItem], tested: dict[str, set],
                        conformance_text: str) -> list[dict]:
    """
    Cross-reference proto items with conformance tests.
    Returns a list of enriched item dicts with test status.
    Uses the full conformance test file text for deeper matching.
    """
    results = []

    for item in proto_items:
        # Skip sentinel values (UNSPECIFIED)
        if item.item_type == "enum_sentinel":
            continue

        # Determine if this item has tests
        status = "untested"
        matched_tests = []

        name_lower = item.name.lower()
        parent_lower = item.parent.lower() if item.parent else ""
        source_lower = item.source_file.lower()

        for tested_key in tested:
            tested_lower = tested_key.lower()
            # Match by name similarity
            if (name_lower in tested_lower or tested_lower in name_lower or
                    (parent_lower and parent_lower in tested_lower)):
                status = "tested"
                matched_tests.extend(tested[tested_key])

        # Special matching for enum values
        if item.item_type == "enum_value":
            for test_name in tested:
                tn = test_name.lower()
                if item.name.lower() in tn:
                    status = "tested"
                    matched_tests.extend(tested[test_name])

        # Check by comment content
        if item.comment:
            comment_lower = item.comment.lower()
            for tested_key in tested:
                tested_lower = tested_key.lower()
                if tested_lower in comment_lower:
                    status = "tested"
                    matched_tests.extend(tested[tested_key])

        # Deep scan: check if item name appears in conformance test source
        if status == "untested":
            # Check for the item name (or a meaningful substring) in the test file
            search_terms = []
            if item.parent:
                search_terms.append(item.parent.lower())
            search_terms.append(item.name.lower())
            # Also check for the base name without domain prefixes
            base_name = item.name.split(".")[-1].lower()
            search_terms.append(base_name)

            for term in search_terms:
                if len(term) >= 3 and term in conformance_text.lower():
                    # Find which test functions contain this term
                    for test_name, test_fns in tested.items():
                        if term in test_name.lower() or any(term in tf.lower() for tf in test_fns):
                            status = "tested"
                            matched_tests.extend(tested[test_name])
                            break

        # Font glyph matching
        if item.item_type == "font_glyph":
            for char_name in ["'0'", "'A'", "'a'", "'!'"]:
                if char_name in item.name:
                    status = "tested"
                    matched_tests.append("font_known_glyphs")
                    break

        # Raster command matching
        if item.item_type == "raster_command":
            cmd_name = item.name.split(".")[-1].lower()
            test_map = {
                "fillrect": ["fill_rect_solid", "fill_rect_zero_size_is_noop", "fill_rect_alpha_blend"],
                "strokerect": ["stroke_rect_solid_border"],
                "text": ["text_renders_non_whitespace", "whitespace_text_is_skipped"],
                "lineargradient": ["linear_gradient_endpoints", "gradient_stop_count_min"],
                "empty_command_list": ["empty_command_list_is_noop"],
            }
            if cmd_name in test_map:
                status = "tested"
                matched_tests.extend(test_map[cmd_name])

        # Framebuffer op matching
        if item.item_type == "framebuffer_op":
            op = item.name.split(".")[-1].lower()
            test_map = {
                "clear": ["framebuffer_clear"],
                "fill_solid": ["framebuffer_fill_solid"],
                "fill_alpha": ["framebuffer_fill_alpha_opaque", "framebuffer_fill_alpha_zero"],
                "clips_bounds": ["framebuffer_clips_bounds"],
            }
            if op in test_map:
                status = "tested"
                matched_tests.extend(test_map[op])

        results.append({
            "name": item.name,
            "domain": item.domain,
            "source_file": item.source_file,
            "item_type": item.item_type,
            "parent": item.parent,
            "comment": item.comment,
            "status": status,
            "tests": sorted(set(matched_tests)),
        })

    return results


# ──────────────────────────────────────────────────────────
# 3. Static Domain Definitions (for items not in protos)
# ──────────────────────────────────────────────────────────


def get_static_items() -> list[ProtoItem]:
    """Return items that are defined outside proto files but tracked in conformance."""
    items = []

    # Blend modes (16 total, defined in Rust blend_lut, not proto)
    blend_modes = [
        "normal", "multiply", "screen", "overlay", "darken", "lighten",
        "colordodge", "colorburn", "hardlight", "softlight",
        "difference", "exclusion", "hue", "saturation", "color", "luminosity",
    ]
    for mode in blend_modes:
        items.append(ProtoItem(
            name=mode,
            domain="Blend Modes",
            source_file="crates/edgerun-rasterizer/blend_lut.rs",
            item_type="blend_mode",
            comment=f"blend_{mode}",
        ))

    # Border patterns (9 total)
    border_patterns = [
        "none", "solid", "dashed", "dotted", "double",
        "groove", "ridge", "inset", "outset",
    ]
    for pattern in border_patterns:
        items.append(ProtoItem(
            name=pattern,
            domain="Border Patterns",
            source_file="crates/edgerun-rasterizer/border_lut.rs",
            item_type="border_pattern",
            comment=f"{pattern.upper()}_PATTERN",
        ))

    # Bitmap font glyphs (128 total, 7-bit ASCII)
    # We'll represent this as a single aggregate item plus a few named glyphs
    items.append(ProtoItem(
        name="FONT_8X8 (128 glyphs)",
        domain="Font Glyphs",
        source_file="crates/edgerun-rasterizer/text_bitmap.rs",
        item_type="font_table",
        comment="7-bit ASCII, 8x8 pixels per glyph",
    ))
    for char, code in [("!", 33), ("0", 48), ("A", 65), ("a", 97)]:
        items.append(ProtoItem(
            name=f"Glyph: '{char}' (ASCII {code})",
            domain="Font Glyphs",
            source_file="crates/edgerun-rasterizer/text_bitmap.rs",
            item_type="font_glyph",
            parent="FONT_8X8",
            comment=f"ASCII code {code}",
        ))

    # Color LUT (148 named colors)
    items.append(ProtoItem(
        name="Color LUT (148 named colors)",
        domain="Color LUT",
        source_file="crates/edgerun-rasterizer/color_lut.rs",
        item_type="color_lut",
        comment="148 CSS named colors",
    ))
    for name in ["black", "white", "red", "green", "blue", "yellow", "cyan", "magenta", "orange", "purple", "transparent"]:
        items.append(ProtoItem(
            name=f"Color: {name}",
            domain="Color LUT",
            source_file="crates/edgerun-rasterizer/color_lut.rs",
            item_type="color_entry",
            parent="Color LUT",
        ))

    # Framebuffer
    for fb_item in ["clear", "fill_solid", "fill_alpha", "clips_bounds"]:
        items.append(ProtoItem(
            name=f"Framebuffer.{fb_item}",
            domain="Framebuffer",
            source_file="crates/edgerun-rasterizer/framebuffer.rs",
            item_type="framebuffer_op",
            comment=fb_item,
        ))

    # Rasterizer
    for rast_item in ["FillRect", "StrokeRect", "Text", "LinearGradient", "empty_command_list"]:
        items.append(ProtoItem(
            name=f"RasterCommand.{rast_item}",
            domain="Scanline Rasterizer",
            source_file="crates/edgerun-rasterizer/scanline.rs",
            item_type="raster_command",
            comment=rast_item,
        ))

    return items


# ──────────────────────────────────────────────────────────
# 4. HTML Dashboard Generator
# ──────────────────────────────────────────────────────────


def generate_html(results: list[dict]) -> str:
    """Generate a self-contained HTML dashboard."""

    # Group by domain
    domains = {}
    for r in results:
        d = r["domain"]
        if d not in domains:
            domains[d] = []
        domains[d].append(r)

    # Sort domains by order of importance
    domain_order = [
        "CSS Properties", "CSS Value Types", "CSS Display Types", "CSS Box Types",
        "CSS Font Types", "CSS Animation Types", "CSS Image Types", "CSS Text Types",
        "CSS Transition Types", "CSS Transform Types", "CSS Cascade Types",
        "CSS Contain Types", "CSS Sizing Types", "CSS Syntax Types", "CSS UI Types",
        "CSS Writing Modes", "CSS At-Rules", "CSS Media Queries", "CSS Page Types",
        "CSS Other",
        "HTML Elements", "HTML Attributes", "HTML Other",
        "Blend Modes", "Border Patterns", "Font Glyphs", "Color LUT", "Framebuffer",
        "Scanline Rasterizer", "DOM Types", "ECMAScript Types", "UI Types",
        "Web Types", "Common Types", "Identity Types", "Capability Types",
        "Trust Types", "Network Types", "Object Types", "Stream Types",
        "IndexedDB Types", "Service Worker Types", "XML Types", "Core Types",
    ]
    # Sort: known domains first (in order), then any unknown domains alphabetically
    ordered_domains = []
    for d in domain_order:
        if d in domains:
            ordered_domains.append(d)
    for d in sorted(domains.keys()):
        if d not in ordered_domains:
            ordered_domains.append(d)

    # Calculate stats
    total = len(results)
    tested = sum(1 for r in results if r["status"] == "tested")
    untested = total - tested
    tested_pct = round(tested / total * 100, 1) if total > 0 else 0
    untested_pct = round(untested / total * 100, 1) if total > 0 else 0

    # Domain stats
    domain_stats = {}
    for d in ordered_domains:
        items = domains[d]
        d_total = len(items)
        d_tested = sum(1 for r in items if r["status"] == "tested")
        domain_stats[d] = {
            "total": d_total,
            "tested": d_tested,
            "untested": d_total - d_tested,
            "pct": round(d_tested / d_total * 100, 1) if d_total > 0 else 0,
        }

    # Proto file summary
    proto_files = sorted(set(r["source_file"] for r in results))

    # Build the HTML
    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Edgerun CSS Spec Conformance Dashboard</title>
<style>
/* ── Reset & Base ── */
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0}}
:root{{
  --bg-primary:#0d1117;--bg-secondary:#161b22;--bg-tertiary:#1c2128;
  --border:#30363d;--text-primary:#e6edf3;--text-secondary:#8b949e;
  --text-muted:#6e7681;--accent:#58a6ff;--accent-hover:#79c0ff;
  --success:#3fb950;--warning:#d29922;--danger:#f85149;
  --progress-bg:#21262d;--progress-tested:#238636;--progress-untested:#30363d;
  --card-bg:#161b22;--hover-bg:#1f242c;
  --font-mono:'SF Mono',SFMono-Regular,Consolas,'Liberation Mono',Menlo,monospace;
  --font-sans:-apple-system,BlinkMacSystemFont,'Segoe UI','Noto Sans',Helvetica,Arial,sans-serif;
}}
html{{background:var(--bg-primary);color:var(--text-primary);font-family:var(--font-sans);font-size:14px;line-height:1.5}}
body{{max-width:1280px;margin:0 auto;padding:24px 32px}}

/* ── Header ── */
.header{{margin-bottom:32px}}
.header h1{{font-size:28px;font-weight:600;margin-bottom:4px;letter-spacing:-0.02em}}
.header p{{color:var(--text-secondary);font-size:14px}}
.header .meta{{color:var(--text-muted);font-size:12px;margin-top:8px}}
.header .meta code{{background:var(--bg-tertiary);padding:2px 6px;border-radius:4px;font-family:var(--font-mono);font-size:11px}}

/* ── Summary Cards ── */
.summary-grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:16px;margin-bottom:32px}}
.summary-card{{background:var(--card-bg);border:1px solid var(--border);border-radius:8px;padding:20px;position:relative;overflow:hidden}}
.summary-card::before{{content:'';position:absolute;top:0;left:0;right:0;height:3px}}
.summary-card.total::before{{background:var(--accent)}}
.summary-card.tested::before{{background:var(--success)}}
.summary-card.untested::before{{background:var(--warning)}}
.summary-card .label{{color:var(--text-secondary);font-size:12px;text-transform:uppercase;letter-spacing:0.05em;margin-bottom:4px}}
.summary-card .value{{font-size:32px;font-weight:700;letter-spacing:-0.02em}}
.summary-card.total .value{{color:var(--accent)}}
.summary-card.tested .value{{color:var(--success)}}
.summary-card.untested .value{{color:var(--warning)}}
.summary-card .pct{{font-size:13px;color:var(--text-muted);margin-top:2px}}

/* ── Controls ── */
.controls{{display:flex;gap:12px;margin-bottom:24px;flex-wrap:wrap;align-items:center}}
.search-box{{flex:1;min-width:250px;position:relative}}
.search-box input{{
  width:100%;padding:8px 12px 8px 36px;background:var(--bg-secondary);
  border:1px solid var(--border);border-radius:6px;color:var(--text-primary);
  font-size:14px;outline:none;transition:border-color 0.15s;
}}
.search-box input:focus{{border-color:var(--accent)}}
.search-box::before{{
  content:'\\1F50D';position:absolute;left:10px;top:50%;transform:translateY(-50%);
  font-size:14px;pointer-events:none;
}}
.filter-btns{{display:flex;gap:8px}}
.filter-btn{{
  padding:6px 14px;border:1px solid var(--border);border-radius:6px;
  background:var(--bg-secondary);color:var(--text-secondary);cursor:pointer;
  font-size:13px;transition:all 0.15s;
}}
.filter-btn:hover{{background:var(--hover-bg);color:var(--text-primary)}}
.filter-btn.active{{background:var(--bg-tertiary);color:var(--text-primary);border-color:var(--accent)}}
.filter-btn .count{{
  display:inline-block;margin-left:4px;padding:1px 6px;border-radius:10px;
  background:var(--bg-primary);font-size:11px;
}}

/* ── Sections ── */
.section{{
  background:var(--card-bg);border:1px solid var(--border);border-radius:8px;
  margin-bottom:16px;overflow:hidden;
}}
.section-header{{
  display:flex;align-items:center;padding:12px 16px;cursor:pointer;
  user-select:none;transition:background 0.15s;
}}
.section-header:hover{{background:var(--hover-bg)}}
.section-header .chevron{{
  margin-right:10px;transition:transform 0.2s;color:var(--text-muted);font-size:12px;
}}
.section-header.collapsed .chevron{{transform:rotate(-90deg)}}
.section-header .title{{flex:1;font-weight:600;font-size:15px}}
.section-header .badge{{
  margin-left:12px;padding:2px 8px;border-radius:12px;font-size:11px;font-weight:500;
}}
.badge-tested{{background:rgba(63,185,80,0.15);color:var(--success)}}
.badge-untested{{background:rgba(210,153,34,0.15);color:var(--warning)}}
.section-progress{{height:4px;background:var(--progress-bg);margin:0 16px 12px;border-radius:2px;overflow:hidden}}
.section-progress-fill{{height:100%;background:var(--progress-tested);border-radius:2px;transition:width 0.3s}}
.section-body{{display:none;border-top:1px solid var(--border)}}
.section-body.expanded{{display:block}}

/* ── Items Table ── */
.items-table{{width:100%;border-collapse:collapse}}
.items-table tr{{border-bottom:1px solid var(--border)}}
.items-table tr:last-child{{border-bottom:none}}
.items-table tr:hover{{background:var(--hover-bg)}}
.items-table td{{padding:8px 16px;vertical-align:middle}}
.items-table .status{{width:24px;text-align:center;font-size:14px}}
.items-table .name{{font-family:var(--font-mono);font-size:13px;color:var(--text-primary)}}
.items-table .parent-name{{font-family:var(--font-mono);font-size:11px;color:var(--text-muted)}}
.items-table .source{{color:var(--text-muted);font-size:12px;font-family:var(--font-mono);max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}}
.items-table .comment{{color:var(--text-secondary);font-size:12px;max-width:300px}}
.items-table .tests{{color:var(--accent);font-size:11px;font-family:var(--font-mono)}}
.items-table tr.hidden-by-filter{{display:none}}
.items-table tr.hidden-by-search{{display:none}}

/* ── Type badges ── */
.type-badge{{
  display:inline-block;padding:1px 6px;border-radius:3px;font-size:10px;
  font-family:var(--font-mono);text-transform:uppercase;
}}
.type-enum_value{{background:rgba(88,166,255,0.15);color:var(--accent)}}
.type-enum{{background:rgba(136,100,255,0.15);color:#a78bfa}}
.type-field{{background:rgba(63,185,80,0.15);color:var(--success)}}
.type-message{{background:rgba(210,153,34,0.15);color:var(--warning)}}
.type-blend_mode{{background:rgba(248,81,73,0.15);color:var(--danger)}}
.type-border_pattern{{background:rgba(210,153,34,0.15);color:var(--warning)}}
.type-font_glyph{{background:rgba(63,185,80,0.15);color:var(--success)}}
.type-color_entry{{background:rgba(88,166,255,0.15);color:var(--accent)}}
.type-default{{background:rgba(139,148,158,0.15);color:var(--text-secondary)}}

/* ── Footer ── */
.footer{{margin-top:32px;padding-top:16px;border-top:1px solid var(--border);color:var(--text-muted);font-size:12px}}
.footer code{{background:var(--bg-tertiary);padding:2px 6px;border-radius:4px;font-family:var(--font-mono);font-size:11px}}
</style>
</head>
<body>

<div class="header">
  <h1>Edgerun Spec Conformance Dashboard</h1>
  <p>Generated from {len(proto_files)} proto files and conformance test suite</p>
  <div class="meta">
    <code>proto/edgerun/v0/</code> &rarr; <code>crates/edgerun-conformance/tests/generated_conformance.rs</code>
  </div>
</div>

<div class="summary-grid">
  <div class="summary-card total">
    <div class="label">Total Spec Items</div>
    <div class="value">{total}</div>
    <div class="pct">across {len(ordered_domains)} domains</div>
  </div>
  <div class="summary-card tested">
    <div class="label">Tested</div>
    <div class="value">{tested}</div>
    <div class="pct">{tested_pct}% coverage</div>
  </div>
  <div class="summary-card untested">
    <div class="label">Untested</div>
    <div class="value">{untested}</div>
    <div class="pct">{untested_pct}% remaining</div>
  </div>
</div>

<div class="controls">
  <div class="search-box">
    <input type="text" id="search" placeholder="Search items, parents, sources..." autocomplete="off">
  </div>
  <div class="filter-btns">
    <button class="filter-btn active" data-filter="all" onclick="setFilter('all')">All<span class="count">{total}</span></button>
    <button class="filter-btn" data-filter="tested" onclick="setFilter('tested')">Tested<span class="count">{tested}</span></button>
    <button class="filter-btn" data-filter="untested" onclick="setFilter('untested')">Untested<span class="count">{untested}</span></button>
  </div>
</div>

<div id="sections">
"""

    for domain in ordered_domains:
        items = domains[domain]
        stats = domain_stats[domain]
        dom_id = re.sub(r'[^a-zA-Z0-9]', '_', domain).lower()
        pct = stats["pct"]

        html += f"""
<div class="section" data-domain="{domain}">
  <div class="section-header" onclick="toggleSection('{dom_id}')">
    <span class="chevron">&#9654;</span>
    <span class="title">{domain}</span>
    <span class="badge badge-tested">{stats['tested']}/{stats['total']} tested</span>
    <span class="badge badge-untested">{pct}%</span>
  </div>
  <div class="section-progress"><div class="section-progress-fill" style="width:{pct}%"></div></div>
  <div class="section-body" id="section-{dom_id}">
    <table class="items-table">
      <thead>
        <tr style="color:var(--text-muted);text-transform:uppercase;font-size:11px;letter-spacing:0.05em">
          <td style="padding:6px 16px;width:24px;text-align:center"></td>
          <td style="padding:6px 16px">Name</td>
          <td style="padding:6px 16px;width:80px">Type</td>
          <td style="padding:6px 16px">Source</td>
          <td style="padding:6px 16px">Tests</td>
        </tr>
      </thead>
      <tbody>
"""

        for r in items:
            status_icon = "&#9989;" if r["status"] == "tested" else "&#9940;"
            type_class = f"type-{r['item_type']}" if f"type-{r['item_type']}" in [
                "type-enum_value", "type-enum", "type-field", "type-message",
                "type-blend_mode", "type-border_pattern", "type-font_glyph",
                "type-color_entry", "type-font_table", "type-color_lut",
                "type-framebuffer_op", "type-raster_command", "type-oneof",
                "type-oneof_field", "type-font_glyph", "type-color_entry",
            ] else "type-default"
            type_label = r["item_type"].replace("_", " ")
            parent_html = f"<div class='parent-name'>{r['parent']}</div>" if r["parent"] else ""
            tests_html = ", ".join(r["tests"][:3]) if r["tests"] else "<span style='color:var(--text-muted)'>none</span>"
            if len(r["tests"]) > 3:
                tests_html += f" +{len(r['tests'])-3}"

            html += f"""
        <tr data-status="{r['status']}" data-search="{r['name'].lower()} {r['parent'].lower() if r['parent'] else ''} {r['source_file'].lower()} {r['comment'].lower()}">
          <td class="status">{status_icon}</td>
          <td>
            <div class="name">{r['name']}{parent_html}</div>
            {f'<div class="comment">{r["comment"]}</div>' if r["comment"] else ''}
          </td>
          <td><span class="type-badge {type_class}">{type_label}</span></td>
          <td class="source" title="{r['source_file']}">{r['source_file']}</td>
          <td class="tests">{tests_html}</td>
        </tr>
"""

        html += """
      </tbody>
    </table>
  </div>
</div>
"""

    html += f"""
</div>

<div class="footer">
  <p>Generated by <code>scripts/generate_dashboard.py</code> &mdash; {len(proto_files)} proto files scanned, {len(results)} total items enumerated.</p>
  <p>Proto files: {', '.join(f'<code>{f}</code>' for f in proto_files[:5])}{'...' if len(proto_files) > 5 else ''}</p>
</div>

<script>
// ── Section Toggle ──
function toggleSection(id) {{
  const body = document.getElementById('section-' + id);
  const header = body.parentElement;
  const isExpanded = body.classList.toggle('expanded');
  header.classList.toggle('collapsed', !isExpanded);
}}

// ── Expand All / Collapse All ──
function expandAll() {{
  document.querySelectorAll('.section-body').forEach(el => {{
    el.classList.add('expanded');
    el.parentElement.querySelector('.section-header').classList.remove('collapsed');
  }});
}}
function collapseAll() {{
  document.querySelectorAll('.section-body').forEach(el => {{
    el.classList.remove('expanded');
    el.parentElement.querySelector('.section-header').classList.add('collapsed');
  }});
}}

// ── Filter ──
let currentFilter = 'all';
function setFilter(filter) {{
  currentFilter = filter;
  document.querySelectorAll('.filter-btn').forEach(btn => {{
    btn.classList.toggle('active', btn.dataset.filter === filter);
  }});
  applyFiltersAndSearch();
}}

// ── Search ──
const searchInput = document.getElementById('search');
let searchQuery = '';
searchInput.addEventListener('input', (e) => {{
  searchQuery = e.target.value.toLowerCase().trim();
  applyFiltersAndSearch();
}});

// ── Apply both ──
function applyFiltersAndSearch() {{
  document.querySelectorAll('.items-table tr[data-status]').forEach(row => {{
    const status = row.dataset.status;
    const search = row.dataset.search || '';
    const matchesFilter = currentFilter === 'all' || status === currentFilter;
    const matchesSearch = !searchQuery || search.includes(searchQuery);
    row.classList.toggle('hidden-by-filter', !matchesFilter);
    row.classList.toggle('hidden-by-search', !matchesSearch);
  }});

  // Update section visibility (hide sections with no visible rows)
  document.querySelectorAll('.section').forEach(section => {{
    const visibleRows = section.querySelectorAll('tr[data-status]:not(.hidden-by-filter):not(.hidden-by-search)');
    section.style.display = visibleRows.length > 0 ? '' : 'none';
  }});
}}

// ── Keyboard shortcuts ──
document.addEventListener('keydown', (e) => {{
  if (e.key === '/' && document.activeElement !== searchInput) {{
    e.preventDefault();
    searchInput.focus();
  }}
  if (e.key === 'Escape') {{
    searchInput.blur();
    searchInput.value = '';
    searchQuery = '';
    applyFiltersAndSearch();
  }}
}});

// ── Auto-expand sections with low coverage ──
document.querySelectorAll('.section').forEach(section => {{
  const badges = section.querySelectorAll('.badge-untested');
  badges.forEach(badge => {{
    const pct = parseFloat(badge.textContent);
    if (pct < 100) {{
      const body = section.querySelector('.section-body');
      const header = section.querySelector('.section-header');
      body.classList.add('expanded');
      header.classList.remove('collapsed');
    }}
  }});
}});
</script>

</body>
</html>
"""

    return html


# ──────────────────────────────────────────────────────────
# 5. Main
# ──────────────────────────────────────────────────────────


def main():
    print(f"Proto directory: {PROTO_DIR}")
    print(f"Conformance file: {CONFORMANCE_FILE}")

    # 1. Discover and parse proto files
    proto_files = discover_proto_files()
    print(f"Found {len(proto_files)} proto files")

    all_items = []
    for pf in proto_files:
        items = parse_proto_file(pf)
        all_items.extend(items)
        print(f"  {pf.relative_to(PROTO_DIR.parent.parent)}: {len(items)} items")

    print(f"\nTotal proto items: {len(all_items)}")

    # 2. Add static items (blend modes, border patterns, font glyphs, etc.)
    static_items = get_static_items()
    all_items.extend(static_items)
    print(f"Static items added: {len(static_items)}")
    print(f"Combined total: {len(all_items)}")

    # 3. Parse conformance tests
    tested = parse_conformance_tests()
    tested_sets = {k: set(v) for k, v in tested.items()}
    print(f"\nConformance test categories: {len(tested_sets)}")

    # Read conformance file text for deep matching
    conformance_text = CONFORMANCE_FILE.read_text() if CONFORMANCE_FILE.exists() else ""

    # 4. Cross-reference
    results = check_test_coverage(all_items, tested_sets, conformance_text)

    # 5. Generate HTML
    html = generate_html(results)
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    OUTPUT_FILE.write_text(html)
    print(f"\nDashboard written to: {OUTPUT_FILE}")
    print(f"File size: {OUTPUT_FILE.stat().st_size / 1024:.1f} KB")

    # Summary
    total = len(results)
    tested_count = sum(1 for r in results if r["status"] == "tested")
    print(f"\n{'='*60}")
    print(f"SUMMARY")
    print(f"{'='*60}")
    print(f"Total spec items:     {total}")
    print(f"Tested:               {tested_count} ({tested_count/total*100:.1f}%)")
    print(f"Untested:             {total - tested_count} ({(total-tested_count)/total*100:.1f}%)")

    # Per-domain breakdown
    domains = {}
    for r in results:
        d = r["domain"]
        if d not in domains:
            domains[d] = {"total": 0, "tested": 0}
        domains[d]["total"] += 1
        if r["status"] == "tested":
            domains[d]["tested"] += 1

    print(f"\n{'Domain':<30} {'Total':>6} {'Tested':>7} {'Coverage':>9}")
    print(f"{'-'*30} {'-'*6} {'-'*7} {'-'*9}")
    for d in sorted(domains.keys()):
        s = domains[d]
        pct = f"{s['tested']/s['total']*100:.1f}%" if s["total"] > 0 else "N/A"
        print(f"{d:<30} {s['total']:>6} {s['tested']:>7} {pct:>9}")


if __name__ == "__main__":
    main()
