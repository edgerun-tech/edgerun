#!/usr/bin/env python3
"""Generate edgerun-webidl, edgerun-flexbox, edgerun-grid, edgerun-color crates."""
import json, os, re, sys

def load(p):
    with open(p) as f: return json.load(f)

def ri(n): return n.replace("-","_")
def rv(n):
    s = n.replace("-","_").replace(" ","_")
    parts = [p.capitalize() for p in s.split("_")]
    return "".join(parts)

def w(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f: f.write(content)

# ===========================================================================
# WEB IDL
# ===========================================================================
def gen_webidl(cat):
    L = ["//! Web IDL type system \u2014 from Web IDL specification.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform2.py",
         "#![cfg_attr(not(test), no_std)]", "",
         "extern crate alloc;",
         "use alloc::{string::String, vec::Vec};", ""]

    # Primitive types
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum PrimitiveType {")
    for t in cat["primitive_types"]:
        L.append(f"    /// {t}")
        L.append(f"    {rv(t)},")
    L.extend(["}", ""])

    # String types
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum StringType {")
    for t in cat["string_types"]:
        L.append(f"    {rv(t)},")
    L.extend(["}", ""])

    # Object types
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum ObjectType {")
    for t in cat["object_types"]:
        L.append(f"    {rv(t)},")
    L.extend(["}", ""])

    # Special types
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum SpecialType {")
    for t in cat["special_types"]:
        L.append(f"    {rv(t)},")
    L.extend(["}", ""])

    # Extended attributes
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum ExtendedAttr {")
    for ea in cat["extended_attributes"]:
        L.append(f"    /// {ea['name']}: {ea['description']}")
        L.append(f"    {rv(ea['name'])},")
    L.extend(["}", ""])

    # WebIDL type union
    L.append("/// A Web IDL type.")
    L.append("#[derive(Debug, Clone, PartialEq)]")
    L.append("pub enum WebIdlType {")
    L.append("    Primitive(PrimitiveType),")
    L.append("    String(StringType),")
    L.append("    Object(ObjectType),")
    L.append("    Special(SpecialType),")
    L.append("    Nullable(Box<WebIdlType>),")
    L.append("    FrozenArray(Box<WebIdlType>),")
    L.append("    Sequence(Box<WebIdlType>),")
    L.append("    Interface(String),")
    L.append("}")

    w(os.path.join("crates/edgerun-webidl/src/lib.rs"), "\n".join(L))
    w(os.path.join("crates/edgerun-webidl/Cargo.toml"),
      '[package]\nname = "edgerun-webidl"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "Web IDL type system generated from W3C spec"\n\n[dependencies]\n')

# ===========================================================================
# FLEXBOX
# ===========================================================================
def gen_flexbox(cat):
    L = ["//! CSS Flexbox types \u2014 from CSS Flexible Box Layout spec.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform2.py",
         "#![cfg_attr(not(test), no_std)]", "",
         "extern crate alloc;",
         "use alloc::string::String;", ""]

    # FlexDirection
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum FlexDirection {")
    for v in ["row", "row-reverse", "column", "column-reverse"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    # FlexWrap
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum FlexWrap {")
    for v in ["nowrap", "wrap", "wrap-reverse"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    # JustifyContent
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum JustifyContent {")
    for v in ["flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly", "start", "end", "left", "right", "normal"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    # AlignContent / AlignItems
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum AlignItems {")
    for v in ["flex-start", "flex-end", "center", "baseline", "stretch", "start", "end", "self-start", "self-end", "first-baseline", "last-baseline", "normal"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    # Flex container
    L.append("/// Flex container properties.")
    L.append("#[derive(Debug, Clone)]")
    L.append("pub struct FlexContainer {")
    L.append("    pub direction: FlexDirection,")
    L.append("    pub wrap: FlexWrap,")
    L.append("    pub justify_content: JustifyContent,")
    L.append("    pub align_content: AlignItems,")
    L.append("    pub align_items: AlignItems,")
    L.append("    pub row_gap: Option<String>,")
    L.append("    pub column_gap: Option<String>,")
    L.extend(["}", ""])

    # Flex item
    L.append("/// Flex item properties.")
    L.append("#[derive(Debug, Clone)]")
    L.append("pub struct FlexItem {")
    L.append("    pub order: i32,")
    L.append("    pub flex_grow: f64,")
    L.append("    pub flex_shrink: f64,")
    L.append("    pub flex_basis: Option<String>,")
    L.append("    pub align_self: Option<AlignItems>,")
    L.extend(["}", ""])

    L.append("impl Default for FlexItem {")
    L.append("    fn default() -> Self {")
    L.append("        Self { order: 0, flex_grow: 0.0, flex_shrink: 1.0, flex_basis: None, align_self: None }")
    L.append("    }")
    L.append("}")

    w(os.path.join("crates/edgerun-flexbox/src/lib.rs"), "\n".join(L))
    w(os.path.join("crates/edgerun-flexbox/Cargo.toml"),
      '[package]\nname = "edgerun-flexbox"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "CSS Flexbox types generated from W3C spec"\n\n[dependencies]\n')

# ===========================================================================
# GRID
# ===========================================================================
def gen_grid(cat):
    L = ["//! CSS Grid types \u2014 from CSS Grid Layout spec.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform2.py",
         "#![cfg_attr(not(test), no_std)]", "",
         "extern crate alloc;",
         "use alloc::string::String;", ""]

    # GridAutoFlow
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum GridAutoFlow {")
    for v in ["row", "column", "dense", "row-dense", "column-dense"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    # Track functions
    L.append("/// Grid track sizing function.")
    L.append("#[derive(Debug, Clone, PartialEq)]")
    L.append("pub enum TrackFunction {")
    L.append("    Minmax(TrackSize, TrackSize),")
    L.append("    FitContent(String),")
    L.append("    Repeat(u32, Vec<TrackSize>),")
    L.append("    MaxContent,")
    L.append("    MinContent,")
    L.append("    Auto,")
    L.append("    Flex(f64),")
    L.append("    Length(String),")
    L.append("}")

    L.append("/// Grid track size.")
    L.append("pub type TrackSize = TrackFunction;")

    # Grid placement
    L.append("/// Grid line placement.")
    L.append("#[derive(Debug, Clone, PartialEq)]")
    L.append("pub enum GridLine {")
    L.append("    Auto,")
    L.append("    Line(i32),")
    L.append("    NamedLine(String),")
    L.append("    Span(u32),")
    L.append("    NamedSpan(String, u32),")
    L.append("}")

    # Grid item
    L.append("/// Grid item placement.")
    L.append("#[derive(Debug, Clone)]")
    L.append("pub struct GridPlacement {")
    L.append("    pub column_start: GridLine,")
    L.append("    pub column_end: GridLine,")
    L.append("    pub row_start: GridLine,")
    L.append("    pub row_end: GridLine,")
    L.append("}")

    # Grid container
    L.append("/// Grid container properties.")
    L.append("#[derive(Debug, Clone)]")
    L.append("pub struct GridContainer {")
    L.append("    pub auto_flow: GridAutoFlow,")
    L.append("    pub auto_columns: Vec<TrackSize>,")
    L.append("    pub auto_rows: Vec<TrackSize>,")
    L.append("    pub row_gap: Option<String>,")
    L.append("    pub column_gap: Option<String>,")
    L.extend(["}", ""])

    # AlignItems for grid
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum GridAlignment {")
    for v in ["start", "end", "center", "stretch", "baseline", "normal", "self-start", "self-end"]:
        L.append(f"    {rv(v)},")
    L.extend(["}", ""])

    w(os.path.join("crates/edgerun-grid/src/lib.rs"), "\n".join(L))
    w(os.path.join("crates/edgerun-grid/Cargo.toml"),
      '[package]\nname = "edgerun-grid"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "CSS Grid types generated from W3C spec"\n\n[dependencies]\n')

# ===========================================================================
# COLOR
# ===========================================================================
def gen_color(cat):
    L = ["//! CSS Color types \u2014 from CSS Color Level 4 specification.",
         "//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform2.py",
         "#![cfg_attr(not(test), no_std)]", "",
         "extern crate alloc;",
         "use alloc::{string::String, vec::Vec};", ""]

    # ColorSpace enum
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum ColorSpace {")
    for cs in cat["color_spaces"]:
        L.append(f"    /// {cs['description']}")
        L.append(f"    {rv(cs['name'])},")
    L.extend(["}", ""])

    # NamedColor enum
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
    L.append("pub enum NamedColor {")
    for c in cat["named_colors"]:
        L.append(f"    {rv(c)},")
    L.extend(["}", ""])

    L.append("impl NamedColor {")
    L.append("    pub fn from_name(name: &str) -> Option<Self> {")
    L.append("        match name {")
    for c in cat["named_colors"]:
        L.append(f'            "{c}" => Some(NamedColor::{rv(c)}),')
    L.extend(['            _ => None,', '        }', '    }', '}', ''])

    # Color value
    L.append("/// A CSS color value.")
    L.append("#[derive(Debug, Clone, PartialEq)]")
    L.append("pub enum CssColor {")
    L.append("    Named(NamedColor),")
    L.append("    Rgb { r: f64, g: f64, b: f64, alpha: f64 },")
    L.append("    Hsl { h: f64, s: f64, l: f64, alpha: f64 },")
    L.append("    Hwb { h: f64, w: f64, b: f64, alpha: f64 },")
    L.append("    Lab { l: f64, a: f64, b: f64, alpha: f64 },")
    L.append("    Lch { l: f64, c: f64, h: f64, alpha: f64 },")
    L.append("    Oklab { l: f64, a: f64, b: f64, alpha: f64 },")
    L.append("    Oklch { l: f64, c: f64, h: f64, alpha: f64 },")
    L.append("    Generic { space: ColorSpace, components: [f64; 3], alpha: f64 },")
    L.append("    CurrentColor,")
    L.append("    Transparent,")
    L.append("}")

    # Interpolation space
    L.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    L.append("pub enum InterpolationSpace {")
    for sp in cat["interpolation_spaces"]:
        L.append(f"    {rv(sp['name'])},")
    L.extend(["}", ""])

    # Color mixing
    L.append("/// Color mix operation.")
    L.append("#[derive(Debug, Clone)]")
    L.append("pub struct ColorMix {")
    L.append("    pub space: InterpolationSpace,")
    L.append("    pub color1: CssColor,")
    L.append("    pub color2: CssColor,")
    L.append("    pub percentage: f64, // 0.0 = color1, 1.0 = color2")
    L.append("}")

    w(os.path.join("crates/edgerun-color/src/lib.rs"), "\n".join(L))
    w(os.path.join("crates/edgerun-color/Cargo.toml"),
      '[package]\nname = "edgerun-color"\nversion = "0.1.0"\nedition.workspace = true\nlicense.workspace = true\npublish = false\ndescription = "CSS Color types generated from W3C Color Level 4"\n\n[dependencies]\n')

# ===========================================================================
def main():
    if len(sys.argv) < 5:
        print(f"Usage: {sys.argv[0]} <webidl.json> <flexbox.json> <grid.json> <color.json>", file=sys.stderr)
        sys.exit(1)

    webidl = load(sys.argv[1])
    flexbox = load(sys.argv[2])
    grid = load(sys.argv[3])
    color = load(sys.argv[4])

    gen_webidl(webidl)
    print(f"  edgerun-webidl: types for {len(webidl['primitive_types'])} primitives, {len(webidl['extended_attributes'])} extended attrs")

    gen_flexbox(flexbox)
    print(f"  edgerun-flexbox: {len(flexbox['container_properties'])} container props, {len(flexbox['child_properties'])} child props")

    gen_grid(grid)
    print(f"  edgerun-grid: {len(grid['properties'])} properties, {len(grid['track_functions'])} track functions")

    gen_color(color)
    print(f"  edgerun-color: {len(color['color_functions'])} functions, {len(color['named_colors'])} named colors, {len(color['color_spaces'])} spaces")

    print("\nDone.")

if __name__ == "__main__":
    main()
