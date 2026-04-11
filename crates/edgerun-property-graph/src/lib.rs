//! CSS Property Knowledge Graph — query the CSS spec as a graph.
//!
//! Parses proto definitions to build a graph of CSS properties with
//! their relationships: inheritance, layout/paint/composite effects,
//! shorthand expansions, and animation behavior.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{string::String, string::ToString, vec::Vec, vec, collections::BTreeMap};

mod property;
mod query;
mod shorthand;

pub use property::*;
pub use query::*;
pub use shorthand::*;

/// The complete knowledge graph of CSS properties.
pub struct PropertyGraph {
    properties: BTreeMap<String, CssProperty>,
    shorthand_map: BTreeMap<String, Vec<String>>,
    reverse_deps: BTreeMap<String, Vec<String>>,
    stats: GraphStats,
}

#[derive(Clone, Debug)]
pub struct GraphStats {
    pub total: usize,
    pub inherited: usize,
    pub animatable: usize,
    pub layout_triggers: usize,
    pub paint_affects: usize,
    pub composite_only: usize,
    pub shorthands: usize,
    pub longhands: usize,
}

impl PropertyGraph {
    /// Build the graph from embedded definitions (no I/O).
    pub fn new() -> Self {
        let mut properties = BTreeMap::new();
        let mut shorthand_map = BTreeMap::new();

        // Populate from proto comments and known definitions
        Self::populate_properties(&mut properties);
        shorthand::build_shorthands(&mut shorthand_map);

        let reverse_deps = Self::build_reverse_deps(&properties);
        let stats = Self::compute_stats(&properties, &shorthand_map);

        Self { properties, shorthand_map, reverse_deps, stats }
    }

    pub fn query(&self) -> PropertyQuery { PropertyQuery::new(self) }
    pub fn get(&self, name: &str) -> Option<&CssProperty> { self.properties.get(name) }
    pub fn property_names(&self) -> Vec<&str> { self.properties.keys().map(|s| s.as_str()).collect() }

    pub fn expand_shorthand(&self, shorthand: &str) -> Vec<&CssProperty> {
        self.shorthand_map.get(shorthand)
            .map(|names| names.iter().filter_map(|n| self.properties.get(n.as_str())).collect())
            .unwrap_or_default()
    }
    pub fn shorthand_names(&self) -> Vec<&str> { self.shorthand_map.keys().map(|s| s.as_str()).collect() }
    pub fn reverse_deps(&self, property: &str) -> Vec<&str> {
        self.reverse_deps.get(property)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
    pub fn stats(&self) -> &GraphStats { &self.stats }
    pub fn len(&self) -> usize { self.properties.len() }
    pub fn is_empty(&self) -> bool { self.properties.is_empty() }

    fn populate_properties(props: &mut BTreeMap<String, CssProperty>) {
        // Layout-triggering properties
        let layout_props = [
            ("width", 99, "css_properties", "<length> | <percentage> | auto", "auto", false, true, true),
            ("min-width", 99, "css_properties", "<length> | <percentage> | auto", "0", false, true, true),
            ("max-width", 100, "css_properties", "<length> | <percentage> | none", "none", false, true, true),
            ("height", 152, "css_properties", "<length> | <percentage> | auto", "auto", false, true, true),
            ("min-height", 153, "css_properties", "<length> | <percentage>", "0", false, true, true),
            ("max-height", 154, "css_properties", "<length> | <percentage> | none", "none", false, true, true),
            ("margin-top", 36, "css_properties", "<length> | <percentage> | auto", "0", false, true, true),
            ("margin-right", 148, "css_properties", "<length> | <percentage> | auto", "0", false, true, true),
            ("margin-bottom", 37, "css_properties", "<length> | <percentage> | auto", "0", false, true, true),
            ("margin-left", 37, "css_properties", "<length> | <percentage> | auto", "0", false, true, true),
            ("padding-top", 39, "css_properties", "<length> | <percentage>", "0", false, true, false),
            ("padding-right", 40, "css_properties", "<length> | <percentage>", "0", false, true, false),
            ("padding-bottom", 40, "css_properties", "<length> | <percentage>", "0", false, true, false),
            ("padding-left", 40, "css_properties", "<length> | <percentage>", "0", false, true, false),
            ("display", 44, "css_properties", "<display>", "inline", false, true, false),
            ("position", 149, "css_properties", "static | relative | absolute | fixed | sticky", "static", false, true, false),
            ("float", 0, "css_properties", "none | left | right | inline-start | inline-end", "none", false, true, false),
            ("clear", 0, "css_properties", "none | left | right | both | inline-start | inline-end", "none", false, true, false),
            ("overflow-x", 93, "css_properties", "visible | hidden | clip | scroll | auto", "visible", false, true, false),
            ("overflow-y", 93, "css_properties", "visible | hidden | clip | scroll | auto", "visible", false, true, false),
            ("grid-template-columns", 81, "css_properties", "<track-list> | <auto-track-list>", "none", false, true, true),
            ("grid-template-rows", 81, "css_properties", "<track-list> | <auto-track-list>", "none", false, true, true),
            ("flex-direction", 46, "css_properties", "row | row-reverse | column | column-reverse", "row", false, true, false),
            ("flex-wrap", 47, "css_properties", "nowrap | wrap | wrap-reverse", "nowrap", false, true, false),
            ("justify-content", 52, "css_properties", "flex-start | flex-end | center | space-between | space-around | space-evenly", "flex-start", false, true, false),
            ("align-items", 53, "css_properties", "flex-start | flex-end | center | baseline | stretch", "stretch", false, true, false),
        ];
        for (name, idx, file, syntax, initial, inherits, layout, paint) in layout_props {
            props.insert(name.into(), CssProperty {
                name: name.into(), proto_idx: idx, proto_file: file.into(),
                value_syntax: syntax.into(), initial_value: initial.into(),
                inherits, affects_layout: layout, affects_paint: paint,
                animatable: true, animation_type: "by-computed-value".into(),
                computed_value: "as-specified".into(),
            });
        }

        // Paint-affecting properties
        let paint_props = [
            ("color", 0, "css_properties", "<color>", "canvastext", true, false, true),
            ("background-color", 10, "css_properties", "<color>", "transparent", false, false, true),
            ("background-image", 11, "css_properties", "<bg-image>#", "none", false, false, true),
            ("border-top-color", 19, "css_properties", "<color>", "currentcolor", false, false, true),
            ("border-right-color", 20, "css_properties", "<color>", "currentcolor", false, false, true),
            ("border-bottom-color", 20, "css_properties", "<color>", "currentcolor", false, false, true),
            ("border-left-color", 20, "css_properties", "<color>", "currentcolor", false, false, true),
            ("outline-color", 127, "css_properties", "<color> | auto", "auto", false, false, true),
            ("box-shadow", 35, "css_properties", "none | <shadow>#", "none", false, false, true),
            ("opacity", 41, "css_properties", "<alpha-value>", "1", false, false, true),
            ("border-top-style", 21, "css_properties", "<line-style>", "none", false, false, true),
            ("border-right-style", 22, "css_properties", "<line-style>", "none", false, false, true),
            ("border-bottom-style", 22, "css_properties", "<line-style>", "none", false, false, true),
            ("border-left-style", 22, "css_properties", "<line-style>", "none", false, false, true),
            ("border-image-source", 29, "css_properties", "none | <image>", "none", false, false, true),
        ];
        for (name, idx, file, syntax, initial, inherits, layout, paint) in paint_props {
            props.insert(name.into(), CssProperty {
                name: name.into(), proto_idx: idx, proto_file: file.into(),
                value_syntax: syntax.into(), initial_value: initial.into(),
                inherits, affects_layout: layout, affects_paint: paint,
                animatable: true, animation_type: "by-computed-value".into(),
                computed_value: "as-specified".into(),
            });
        }

        // Inherited properties (font/text related)
        let inherited_props = [
            ("font-size", 60, "css_properties", "<absolute-size> | <relative-size> | <length>", "medium", true, true, true),
            ("font-family", 56, "css_properties", "[<family-name> | <generic-family>]#", "sans-serif", true, false, false),
            ("font-weight", 57, "css_properties", "<font-weight-absolute> | bolder | lighter", "normal", true, false, false),
            ("font-style", 59, "css_properties", "normal | italic | oblique", "normal", true, false, false),
            ("line-height", 155, "css_properties", "normal | <number> | <length> | <percentage>", "normal", true, false, false),
            ("letter-spacing", 114, "css_properties", "normal | <length>", "normal", true, false, false),
            ("word-spacing", 113, "css_properties", "normal | <length>", "normal", true, false, false),
            ("text-align", 109, "css_properties", "start | end | left | right | center | justify", "start", true, false, false),
            ("text-indent", 115, "css_properties", "<length-percentage>", "0", true, false, false),
            ("text-transform", 102, "css_properties", "none | capitalize | uppercase | lowercase | full-width", "none", true, false, false),
            ("white-space", 103, "css_properties", "normal | pre | nowrap | pre-wrap | break-spaces", "normal", true, false, false),
            ("word-break", 105, "css_properties", "normal | keep-all | break-all | break-word", "normal", true, false, false),
            ("text-overflow", 98, "css_properties", "clip | ellipsis | <string>", "clip", false, false, true),
            ("writing-mode", 144, "css_properties", "horizontal-tb | vertical-rl | vertical-lr", "horizontal-tb", true, true, false),
            ("direction", 142, "css_properties", "ltr | rtl", "ltr", true, true, false),
        ];
        for (name, idx, file, syntax, initial, inherits, layout, paint) in inherited_props {
            props.insert(name.into(), CssProperty {
                name: name.into(), proto_idx: idx, proto_file: file.into(),
                value_syntax: syntax.into(), initial_value: initial.into(),
                inherits, affects_layout: layout, affects_paint: paint,
                animatable: true, animation_type: "by-computed-value".into(),
                computed_value: "as-specified".into(),
            });
        }

        // Animation/transition properties (composite-level)
        let composite_props = [
            ("transform", 117, "css_properties", "none | <transform-list>", "none", false, false, false),
            ("transition-property", 120, "css_properties", "none | <single-transition-property>#", "all", false, false, false),
            ("transition-duration", 121, "css_properties", "<time>#", "0s", false, false, false),
            ("animation-duration", 2, "css_properties", "<time>#", "0s", false, false, false),
            ("animation-name", 1, "css_properties", "none | <keyframes-name>#", "none", false, false, false),
            ("will-change", 0, "css_properties", "auto | <animateable-feature>#", "auto", false, false, false),
        ];
        for (name, idx, file, syntax, initial, inherits, layout, paint) in composite_props {
            props.insert(name.into(), CssProperty {
                name: name.into(), proto_idx: idx, proto_file: file.into(),
                value_syntax: syntax.into(), initial_value: initial.into(),
                inherits, affects_layout: layout, affects_paint: paint,
                animatable: false, animation_type: "discrete".into(),
                computed_value: "as-specified".into(),
            });
        }

        // Discrete (non-animatable) properties
        let discrete_props = [
            ("display", 44, "css_properties", "<display>", "inline", false, true, true),
            ("position", 149, "css_properties", "static | relative | absolute | fixed | sticky", "static", false, true, false),
            ("overflow", 94, "css_properties", "visible | hidden | clip | scroll | auto", "visible", false, true, true),
            ("visibility", 45, "css_properties", "visible | hidden | collapse", "visible", false, false, true),
            ("pointer-events", 136, "css_properties", "auto | none", "auto", false, false, false),
            ("user-select", 135, "css_properties", "auto | text | none | contain | all", "auto", true, false, false),
            ("resize", 129, "css_properties", "none | both | horizontal | vertical | block | inline", "none", false, false, false),
            ("cursor", 130, "css_properties", "[<cursor-image>]* <cursor-predefined>", "auto", true, false, false),
            ("appearance", 141, "css_properties", "none | auto | base | base-select", "auto", false, false, false),
        ];
        for (name, idx, file, syntax, initial, inherits, layout, paint) in discrete_props {
            // Don't overwrite existing entries
            if !props.contains_key(name) {
                props.insert(name.into(), CssProperty {
                    name: name.into(), proto_idx: idx, proto_file: file.into(),
                    value_syntax: syntax.into(), initial_value: initial.into(),
                    inherits, affects_layout: layout, affects_paint: paint,
                    animatable: false, animation_type: "discrete".into(),
                    computed_value: "as-specified".into(),
                });
            }
        }

        // Additional CSS properties from proto (no detailed metadata yet)
        let extra = [
            "animation", "animation-delay", "animation-direction", "animation-fill-mode",
            "animation-iteration-count", "animation-play-state", "animation-timing-function",
            "background", "background-attachment", "background-clip", "background-origin",
            "background-position", "background-repeat", "background-size",
            "border", "border-collapse", "border-image", "border-image-outset",
            "border-image-repeat", "border-image-slice", "border-image-width",
            "border-spacing", "border-top", "border-top-left-radius", "border-top-right-radius",
            "bottom", "box-sizing", "caption-side", "contain", "content", "content-visibility",
            "counter-increment", "counter-reset", "empty-cells", "flex", "flex-basis",
            "flex-flow", "flex-grow", "flex-shrink", "font", "font-feature-settings",
            "font-kerning", "font-language-override", "font-optical-sizing",
            "font-palette", "font-size-adjust", "font-stretch", "font-synthesis",
            "font-synthesis-position", "font-synthesis-small-caps", "font-synthesis-style",
            "font-synthesis-weight", "font-variant", "font-variant-alternates",
            "font-variant-caps", "font-variant-east-asian", "font-variant-emoji",
            "font-variant-ligatures", "font-variant-numeric", "font-variant-position",
            "font-variation-settings", "grid", "grid-area", "grid-auto-columns",
            "grid-auto-flow", "grid-auto-rows", "grid-column", "grid-row", "grid-template",
            "hanging-punctuation", "hyphens", "image-orientation", "image-rendering",
            "interest-delay", "interest-delay-start", "interactivity", "isolation",
            "left", "letter-spacing", "line-break", "list-style", "list-style-image",
            "list-style-position", "list-style-type", "margin", "margin-trim", "max-height",
            "max-width", "min-height", "min-width", "mix-blend-mode", "nav-down",
            "nav-left", "nav-right", "nav-up", "object-fit", "object-position", "orphans",
            "outline-offset", "outline-style", "outline-width", "overflow-block",
            "overflow-clip-margin", "overflow-inline", "overflow-wrap", "overflow-x",
            "overflow-y", "page-break-after", "page-break-before", "page-break-inside",
            "perspective", "perspective-origin", "quotes", "right", "rotate",
            "row-gap", "scale", "scroll-behavior", "scroll-margin",
            "scroll-margin-block", "scroll-margin-block-end", "scroll-margin-block-start",
            "scroll-margin-bottom", "scroll-margin-inline", "scroll-margin-inline-end",
            "scroll-margin-inline-start", "scroll-margin-left", "scroll-margin-right",
            "scroll-margin-top", "scroll-padding", "scroll-padding-block",
            "scroll-padding-block-end", "scroll-padding-block-start", "scroll-padding-bottom",
            "scroll-padding-inline", "scroll-padding-inline-end", "scroll-padding-inline-start",
            "scroll-padding-left", "scroll-padding-right", "scroll-padding-top",
            "scroll-snap-align", "scroll-snap-stop", "scroll-snap-type",
            "scrollbar-color", "scrollbar-gutter", "scrollbar-width", "tab-size",
            "table-layout", "text-align-all", "text-align-last", "text-combine-upright",
            "text-decoration", "text-decoration-color", "text-decoration-line",
            "text-decoration-style", "text-emphasis", "text-emphasis-color",
            "text-emphasis-position", "text-emphasis-style", "text-justify",
            "text-orientation", "text-underline-offset", "text-underline-position",
            "top", "touch-action", "transform-box", "transform-origin", "transform-style",
            "transition", "transition-delay", "transition-duration",
            "transition-timing-function", "translate", "unicode-bidi", "vertical-align",
            "visibility", "white-space", "widows", "word-break", "word-spacing",
            "writing-mode", "z-index",
        ];
        for (i, name) in extra.iter().enumerate() {
            if !props.contains_key(*name) {
                props.insert(name.to_string(), CssProperty {
                    name: name.to_string(),
                    proto_idx: i as u32 + 176,
                    proto_file: "css_properties".into(),
                    value_syntax: String::new(),
                    initial_value: String::new(),
                    inherits: false,
                    affects_layout: false,
                    affects_paint: false,
                    animatable: true,
                    animation_type: "by-computed-value".into(),
                    computed_value: String::new(),
                });
            }
        }
    }

    fn build_reverse_deps(props: &BTreeMap<String, CssProperty>) -> BTreeMap<String, Vec<String>> {
        let mut map = BTreeMap::new();
        // Known dependency relationships from CSS spec
        let deps = [
            ("font-size", "line-height"),
            ("font-size", "font-stretch"),
            ("font-size", "text-indent"),
            ("font-size", "letter-spacing"),
            ("line-height", "vertical-align"),
            ("color", "border-top-color"),
            ("color", "border-right-color"),
            ("color", "border-bottom-color"),
            ("color", "border-left-color"),
            ("color", "outline-color"),
            ("color", "text-decoration-color"),
            ("color", "caret-color"),
            ("font-family", "font-size"),
            ("font-weight", "font-size"),
            ("display", "width"),
            ("display", "height"),
            ("display", "margin-top"),
            ("display", "margin-bottom"),
            ("position", "top"),
            ("position", "right"),
            ("position", "bottom"),
            ("position", "left"),
            ("position", "z-index"),
            ("overflow-x", "overflow-y"),
            ("background-color", "background-image"),
        ];
        for (parent, child) in deps {
            if props.contains_key(parent) && props.contains_key(child) {
                map.entry(parent.to_string())
                    .or_insert_with(Vec::new)
                    .push(child.to_string());
            }
        }
        map
    }

    fn compute_stats(props: &BTreeMap<String, CssProperty>, shorthand_map: &BTreeMap<String, Vec<String>>) -> GraphStats {
        let mut inherited = 0;
        let mut animatable = 0;
        let mut layout_triggers = 0;
        let mut paint_affects = 0;
        let mut composite_only = 0;
        let mut longhands = 0;
        for p in props.values() {
            if p.inherits { inherited += 1; }
            if p.animatable { animatable += 1; }
            if p.affects_layout { layout_triggers += 1; }
            if p.affects_paint { paint_affects += 1; }
            if !p.affects_layout && !p.affects_paint { composite_only += 1; }
        }
        for v in shorthand_map.values() { longhands += v.len(); }
        GraphStats {
            total: props.len(),
            inherited, animatable, layout_triggers, paint_affects, composite_only,
            shorthands: shorthand_map.len(),
            longhands,
        }
    }
}

impl Default for PropertyGraph {
    fn default() -> Self { Self::new() }
}
