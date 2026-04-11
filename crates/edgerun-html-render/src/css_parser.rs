//! CSS Parser — generated from css_properties.proto + css_value_types.proto.
//!
//! Generated from:
//! - 255 CSS property definitions (edgerun.v0.css.properties)
//! - CSS value type definitions (edgerun.v0.css.value_types)
extern crate alloc;

use alloc::string::{String, ToString};

use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// CSS property name → value string.
pub type Declarations = BTreeMap<String, String>;

/// A CSS rule: selector → declarations.
#[derive(Debug, Clone)]
pub struct Rule {
    pub selector: String,
    pub declarations: Declarations,
}

/// Stylesheet = list of rules.
#[derive(Debug, Clone)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn new() -> Self { Self { rules: Vec::new() } }

    /// Get computed style for a tag + class + id combination.
    pub fn compute(&self, tag: &str, class: Option<&str>, id: Option<&str>) -> Declarations {
        let mut result = Declarations::new();
        for rule in &self.rules {
            if selector_matches(&rule.selector, tag, class, id) {
                for (k, v) in &rule.declarations {
                    result.insert(k.clone(), v.clone());
                }
            }
        }
        result
    }
}

/// Known CSS property names — generated from css_properties.proto.
pub const KNOWN_PROPERTIES: &[&str] = &["accent-color", "align-content", "align-items", "align-self", "all", "animation", "animation-delay", "animation-direction", "animation-duration", "animation-fill-mode", "animation-iteration-count", "animation-name", "animation-play-state", "animation-timing-function", "appearance", "aspect-ratio", "backface-visibility", "background", "background-attachment", "background-clip", "background-color", "background-image", "background-origin", "background-position", "background-repeat", "background-size", "block-size", "border", "border-bottom", "border-bottom-color", "border-bottom-left-radius", "border-bottom-right-radius", "border-bottom-style", "border-bottom-width", "border-collapse", "border-color", "border-image", "border-image-outset", "border-image-repeat", "border-image-slice", "border-image-source", "border-image-width", "border-left", "border-radius", "border-right", "border-spacing", "border-style", "border-top", "border-top-color", "border-top-left-radius", "border-top-right-radius", "border-top-style", "border-top-width", "border-width", "bottom", "box-decoration-break", "box-shadow", "box-sizing", "break-after", "break-before", "break-inside", "caption-side", "caret-color", "clear", "clip", "clip-path", "color", "column-count", "column-fill", "column-gap", "column-rule", "column-rule-color", "column-rule-style", "column-rule-width", "column-span", "column-width", "columns", "contain", "content", "counter-increment", "counter-reset", "cursor", "direction", "display", "empty-cells", "flex", "flex-basis", "flex-direction", "flex-flow", "flex-grow", "flex-shrink", "flex-wrap", "float", "font", "font-family", "font-feature-settings", "font-kerning", "font-size", "font-size-adjust", "font-stretch", "font-style", "font-synthesis", "font-variant", "font-variant-caps", "font-variant-ligatures", "font-variant-numeric", "font-variation-settings", "font-weight", "gap", "grid", "grid-area", "grid-auto-columns", "grid-auto-flow", "grid-auto-rows", "grid-column", "grid-column-end", "grid-column-start", "grid-row", "grid-row-end", "grid-row-start", "grid-template", "grid-template-areas", "grid-template-columns", "grid-template-rows", "height", "hyphens", "image-orientation", "image-rendering", "isolation", "justify-content", "justify-items", "justify-self", "left", "letter-spacing", "line-break", "line-height", "list-style", "list-style-image", "list-style-position", "list-style-type", "margin", "margin-bottom", "margin-left", "margin-right", "margin-top", "mask", "mask-clip", "mask-image", "mask-origin", "mask-position", "mask-repeat", "mask-size", "max-height", "max-width", "min-height", "min-width", "mix-blend-mode", "object-fit", "object-position", "opacity", "order", "orphans", "outline", "outline-color", "outline-offset", "outline-style", "outline-width", "overflow", "overflow-wrap", "overflow-x", "overflow-y", "padding", "padding-bottom", "padding-left", "padding-right", "padding-top", "page-break-after", "page-break-before", "page-break-inside", "perspective", "perspective-origin", "place-content", "place-items", "place-self", "pointer-events", "position", "quotes", "resize", "right", "rotate", "row-gap", "ruby-align", "ruby-position", "scale", "scroll-behavior", "scroll-margin", "scroll-padding", "scroll-snap-align", "scroll-snap-type", "shape-margin", "shape-outside", "speak-as", "stop-color", "stroke", "stroke-dasharray", "stroke-dashoffset", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "stroke-opacity", "stroke-width", "tab-size", "table-layout", "text-align", "text-align-last", "text-anchor", "text-decoration", "text-decoration-color", "text-decoration-line", "text-decoration-style", "text-decoration-thickness", "text-indent", "text-justify", "text-orientation", "text-overflow", "text-rendering", "text-shadow", "text-transform", "text-underline-position", "top", "touch-action", "transform", "transform-box", "transform-origin", "transform-style", "transition", "transition-delay", "transition-duration", "transition-property", "transition-timing-function", "translate", "unicode-bidi", "user-select", "vertical-align", "visibility", "white-space", "widows", "width", "will-change", "word-break", "word-spacing", "word-wrap", "writing-mode", "z-index", "zoom"];

/// Known CSS value keywords — generated from css_value_types.proto.
pub const KNOWN_KEYWORDS: &[&str] = &["absolute", "alias", "all", "all-scroll", "alternate", "alternate-reverse", "always", "anywhere", "auto", "avoid", "avoid-page", "backwards", "baseline", "block", "bold", "border-box", "both", "bottom", "break-all", "break-spaces", "break-word", "capitalize", "cell", "center", "clip", "col-resize", "column", "column-reverse", "contain", "content-box", "contents", "context-menu", "copy", "cover", "crosshair", "currentColor", "dashed", "default", "dotted", "double", "ease", "ease-in", "ease-in-out", "ease-out", "ellipsis", "end", "fill", "fixed", "flex", "flow-root", "forwards", "grab", "grabbing", "grid", "groove", "help", "hidden", "horizontal-tb", "infinite", "inherit", "initial", "inline", "inline-block", "inset", "italic", "keep-all", "landscape", "left", "linear", "list-item", "lowercase", "middle", "mixed", "move", "no-drop", "none", "normal", "not-allowed", "nowrap", "outset", "page", "paused", "pointer", "portrait", "pre", "pre-line", "pre-wrap", "progress", "recto", "relative", "reverse", "revert", "ridge", "right", "row", "row-resize", "row-reverse", "ruby", "running", "scale-down", "scroll", "sideways", "solid", "space-around", "space-between", "space-evenly", "start", "step-end", "step-start", "sticky", "stretch", "sub", "super", "table", "table-cell", "table-footer-group", "table-header-group", "table-row", "table-row-group", "text", "text-bottom", "text-top", "top", "transparent", "underline", "unset", "uppercase", "upright", "verso", "vertical-lr", "vertical-rl", "vertical-text", "visible", "wait", "wrap", "wrap-reverse", "zoom-in", "zoom-out"];

/// Check if a property name is valid CSS.
pub fn is_valid_property(name: &str) -> bool {
    KNOWN_PROPERTIES.contains(&name)
}

/// Parse a CSS stylesheet string.
pub fn parse_css(css: &str) -> Stylesheet {
    let mut stylesheet = Stylesheet::new();
    let mut parser = CssParser { input: css, pos: 0 };
    while let Some(rule) = parser.parse_rule() {
        stylesheet.rules.push(rule);
    }
    stylesheet
}

struct CssParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> CssParser<'a> {
    fn parse_rule(&mut self) -> Option<Rule> {
        self.skip_ws_and_comments();
        if self.pos >= self.input.len() { return None; }

        // Read selector
        let selector = self.read_while(|c| c != '{');
        if selector.trim().is_empty() { return None; }
        self.skip_ws();

        if self.pos >= self.input.len() || self.input.as_bytes()[self.pos] != b'{' {
            return None;
        }
        self.pos += 1;

        // Parse declarations
        let mut decls = Declarations::new();
        loop {
            self.skip_ws();
            if self.pos >= self.input.len() || self.input.as_bytes()[self.pos] == b'}' {
                break;
            }
            let prop = self.read_while(|c| c != ':' && c != '}' && c != ';');
            self.skip_ws();
            if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b':' {
                self.pos += 1;
            }
            let value = self.read_while(|c| c != ';' && c != '}');
            if !prop.trim().is_empty() && !value.trim().is_empty() {
                decls.insert(prop.trim().to_string(), value.trim().to_string());
            }
            if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b';' {
                self.pos += 1;
            }
        }

        if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b'}' {
            self.pos += 1;
        }

        Some(Rule {
            selector: selector.trim().to_string(),
            declarations: decls,
        })
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            self.skip_ws();
            if self.pos + 1 < self.input.len() && &self.input[self.pos..self.pos+2] == "/*" {
                if let Some(end) = self.input[self.pos..].find("*/") {
                    self.pos += end + 2;
                    continue;
                }
            }
            break;
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn read_while<F: Fn(char) -> bool>(&mut self, f: F) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            if let Some(c) = self.input[self.pos..].chars().next() {
                if !f(c) { break; }
                self.pos += c.len_utf8();
            } else { break; }
        }
        self.input[start..self.pos].to_string()
    }
}

/// Simple selector matching.
fn selector_matches(selector: &str, tag: &str, class: Option<&str>, id: Option<&str>) -> bool {
    for sel in selector.split(',') {
        let sel = sel.trim();
        if selector_match_one(sel, tag, class, id) {
            return true;
        }
    }
    false
}

fn selector_match_one(selector: &str, tag: &str, class: Option<&str>, id: Option<&str>) -> bool {
    // #id
    if let Some(sel_id) = selector.strip_prefix('#') {
        return id == Some(sel_id);
    }
    // .class
    if let Some(sel_class) = selector.strip_prefix('.') {
        return class == Some(sel_class);
    }
    // element.class
    if selector.contains('.') {
        let parts: Vec<&str> = selector.splitn(2, '.').collect();
        if parts[0] != "*" && parts[0].to_lowercase() != tag {
            return false;
        }
        return class == Some(parts[1]);
    }
    // element or universal
    selector.to_lowercase() == tag || selector == "*"
}
