//! Proto file parser — extracts CSS property definitions from proto sources.
//!
//! This module parses the raw text of `.proto` files (no protobuf runtime needed)
//! to extract `CssProperty` enum entries and their `///` doc comments.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::graph::PropertyNode;

// ---------------------------------------------------------------------------
// Shorthand → longhand mappings (authoritative CSS spec knowledge)
// ---------------------------------------------------------------------------

/// Returns the longhand properties for a given CSS shorthand.
fn shorthand_longhands(shorthand: &str) -> Option<Vec<&'static str>> {
    match shorthand {
        "animation" => Some(vec![
            "animation-name", "animation-duration", "animation-timing-function",
            "animation-delay", "animation-iteration-count", "animation-direction",
            "animation-fill-mode", "animation-play-state",
        ]),
        "background" => Some(vec![
            "background-color", "background-image", "background-repeat",
            "background-attachment", "background-position", "background-clip",
            "background-origin", "background-size",
        ]),
        "border" => Some(vec![
            "border-top-width", "border-right-width", "border-bottom-width", "border-left-width",
            "border-top-style", "border-right-style", "border-bottom-style", "border-left-style",
            "border-top-color", "border-right-color", "border-bottom-color", "border-left-color",
        ]),
        "border-top" => Some(vec!["border-top-width", "border-top-style", "border-top-color"]),
        "border-color" => Some(vec![
            "border-top-color", "border-right-color", "border-bottom-color", "border-left-color",
        ]),
        "border-style" => Some(vec![
            "border-top-style", "border-right-style", "border-bottom-style", "border-left-style",
        ]),
        "border-width" => Some(vec![
            "border-top-width", "border-right-width", "border-bottom-width", "border-left-width",
        ]),
        "border-radius" => Some(vec![
            "border-top-left-radius", "border-top-right-radius",
            "border-bottom-right-radius", "border-bottom-left-radius",
        ]),
        "border-image" => Some(vec![
            "border-image-source", "border-image-slice", "border-image-width",
            "border-image-outset", "border-image-repeat",
        ]),
        "margin" => Some(vec!["margin-top", "margin-right", "margin-bottom", "margin-left"]),
        "padding" => Some(vec!["padding-top", "padding-right", "padding-bottom", "padding-left"]),
        "font" => Some(vec![
            "font-style", "font-variant", "font-weight", "font-width", "font-size",
            "line-height", "font-family",
        ]),
        "flex" => Some(vec!["flex-grow", "flex-shrink", "flex-basis"]),
        "flex-flow" => Some(vec!["flex-direction", "flex-wrap"]),
        "grid" => Some(vec![
            "grid-template-rows", "grid-template-columns", "grid-template-areas",
            "grid-auto-rows", "grid-auto-columns", "grid-auto-flow",
        ]),
        "grid-template" => Some(vec![
            "grid-template-rows", "grid-template-columns", "grid-template-areas",
        ]),
        "grid-row" => Some(vec!["grid-row-start", "grid-row-end"]),
        "grid-column" => Some(vec!["grid-column-start", "grid-column-end"]),
        "grid-area" => Some(vec!["grid-row-start", "grid-column-start", "grid-row-end", "grid-column-end"]),
        "gap" => Some(vec!["row-gap", "column-gap"]),
        "list-style" => Some(vec!["list-style-type", "list-style-position", "list-style-image"]),
        "outline" => Some(vec!["outline-color", "outline-style", "outline-width"]),
        "transition" => Some(vec![
            "transition-property", "transition-duration",
            "transition-timing-function", "transition-delay",
        ]),
        "overflow" => Some(vec!["overflow-x", "overflow-y"]),
        "place-items" => Some(vec!["align-items", "justify-items"]),
        "place-content" => Some(vec!["align-content", "justify-content"]),
        "place-self" => Some(vec!["align-self", "justify-self"]),
        "text-decoration" => Some(vec![
            "text-decoration-line", "text-decoration-color",
            "text-decoration-style", "text-decoration-thickness",
        ]),
        "inset" => Some(vec!["top", "right", "bottom", "left"]),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Layout / paint inference
// ---------------------------------------------------------------------------

/// Properties that are known to affect layout when changed.
fn affects_layout(name: &str) -> bool {
    matches!(
        name,
        "width" | "height"
            | "min-width" | "min-height"
            | "max-width" | "max-height"
            | "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" | "margin-trim"
            | "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left"
            | "display"
            | "position"
            | "top" | "right" | "bottom" | "left"
            | "float" | "clear"
            | "grid-template-columns" | "grid-template-rows" | "grid-template-areas"
            | "grid-auto-columns" | "grid-auto-rows" | "grid-auto-flow"
            | "grid-template"
            | "grid-row" | "grid-column" | "grid-area"
            | "flex-direction" | "flex-wrap" | "flex-flow"
            | "flex-grow" | "flex-shrink" | "flex-basis"
            | "justify-content" | "align-items" | "align-content"
            | "gap"
            | "box-sizing"
            | "contain"
            | "content-visibility"
            | "writing-mode" | "direction" | "text-orientation"
            | "vertical-align"
            | "line-height"
            | "font-size"
            | "font-family"
            | "font-weight"
            | "font-style"
            | "overflow" | "overflow-x" | "overflow-y"
            | "scroll-behavior"
            | "visibility"
            | "white-space"
            | "tab-size"
            | "word-break" | "line-break" | "overflow-wrap" | "hyphens"
            | "text-align" | "text-align-all" | "text-align-last"
            | "text-indent"
            | "text-transform"
            | "word-spacing" | "letter-spacing"
            | "object-fit" | "object-position"
            | "transform"
            | "page-break-before" | "page-break-after" | "page-break-inside"
            | "orphans" | "widows"
            | "table-layout"
            | "border-collapse" | "border-spacing"
            | "caption-side"
            | "list-style" | "list-style-type" | "list-style-position" | "list-style-image"
            | "counter-reset" | "counter-increment"
            | "quotes"
            | "content"
            | "empty-cells"
    )
}

/// Properties that are known to affect paint (visual rendering) when changed.
fn affects_paint(name: &str) -> bool {
    matches!(
        name,
        "background" | "background-color" | "background-image" | "background-repeat"
            | "background-attachment" | "background-position" | "background-clip"
            | "background-origin" | "background-size"
            | "color"
            | "border" | "border-top" | "border-color" | "border-style" | "border-width"
            | "border-top-color" | "border-top-style" | "border-top-width"
            | "border-radius" | "border-top-left-radius"
            | "border-image" | "border-image-source" | "border-image-slice"
            | "border-image-width" | "border-image-outset" | "border-image-repeat"
            | "border-collapse" | "border-spacing"
            | "box-shadow"
            | "opacity"
            | "outline" | "outline-color" | "outline-style" | "outline-width" | "outline-offset"
            | "text-decoration"
            | "text-shadow"
            | "box-decoration-break"
            | "filter"
            | "mix-blend-mode"
            | "isolation"
            | "mask" | "mask-image" | "mask-mode" | "mask-repeat" | "mask-position"
            | "mask-clip" | "mask-origin" | "mask-size" | "mask-composite"
            | "clip-path"
            | "image-orientation" | "image-rendering"
            | "appearance"
            | "accent-color"
            | "cursor"
            | "caret-color" | "caret-animation" | "caret-shape"
            | "font-variant-ligatures" | "font-variant-caps" | "font-variant-numeric"
            | "font-variant-position" | "font-variant-east-asian" | "font-variant"
            | "font-feature-settings" | "font-variation-settings" | "font-kerning"
            | "font-optical-sizing"
            | "font-palette"
            | "text-overflow"
            | "text-combine-upright"
            | "empty-cells"
            | "list-style" | "list-style-type" | "list-style-image" | "list-style-position"
    )
}

/// Properties that are animatable per CSS spec.
fn animatable(name: &str) -> bool {
    // Animation properties are inherently animatable
    if name.starts_with("animation-") {
        return true;
    }
    // Transition properties themselves are not animatable
    if name.starts_with("transition-") {
        return false;
    }
    matches!(
        name,
        "opacity"
            | "background-color"
            | "color"
            | "border-color" | "border-top-color" | "border-right-color" | "border-bottom-color" | "border-left-color"
            | "outline-color"
            | "box-shadow"
            | "text-shadow"
            | "width" | "height" | "min-width" | "min-height" | "max-width" | "max-height"
            | "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left"
            | "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left"
            | "top" | "right" | "bottom" | "left"
            | "font-size"
            | "line-height"
            | "letter-spacing" | "word-spacing"
            | "text-indent"
            | "transform" | "transform-origin"
            | "filter"
            | "clip-path"
            | "background-position" | "background-size"
            | "border-width" | "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width"
            | "border-radius" | "border-top-left-radius" | "border-top-right-radius"
            | "border-bottom-right-radius" | "border-bottom-left-radius"
            | "outline-width" | "outline-offset"
            | "visibility"
            | "z-index"
            | "flex-grow" | "flex-shrink" | "flex-basis"
            | "grid-row" | "grid-column" | "grid-area"
            | "gap"
            | "object-position"
            | "vertical-align"
            | "scroll-behavior"
            | "caret-color"
            | "accent-color"
    )
}

// ---------------------------------------------------------------------------
// Proto parsing
// ---------------------------------------------------------------------------

/// Convert a SCREAMING_SNAKE_CASE enum name to kebab-case property name.
fn enum_to_property_name(enum_name: &str) -> String {
    let mut result = String::new();
    let lower = enum_name.to_lowercase();
    for (i, ch) in lower.chars().enumerate() {
        if ch == '_' {
            result.push('-');
        } else {
            if i > 0 {
                let chars: Vec<char> = lower.chars().collect();
                if i > 0 && chars.get(i.saturating_sub(1)) == Some(&'_') {
                    // already handled above
                }
            }
            result.push(ch);
        }
    }
    result
}

/// Extract doc comment from a line if it starts with `//`.
fn extract_comment(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        Some(trimmed.strip_prefix("//").unwrap_or("").trim())
    } else {
        None
    }
}

/// Extract an enum entry name and its numeric value from a line like `ANIMATION_NAME = 1;`.
fn parse_enum_entry(line: &str) -> Option<(&str, u32)> {
    let trimmed = line.trim().trim_end_matches(',').trim_end_matches(';');
    let parts: Vec<&str> = trimmed.split('=').collect();
    if parts.len() == 2 {
        let name = parts[0].trim();
        let val_str = parts[1].trim();
        if let Ok(val) = val_str.parse::<u32>() {
            if name != "CSS_PROPERTY_UNSPECIFIED" {
                return Some((name, val));
            }
        }
    }
    None
}

/// Parse the value syntax from a proto comment.
///
/// Comments look like: `// font-size: <absolute-size> | <relative-size> | <length-percentage [0,∞]> | math`
/// Returns the syntax portion after the colon.
fn parse_value_syntax(comment: &str) -> &str {
    if let Some(pos) = comment.find(':') {
        comment[pos + 1..].trim()
    } else {
        comment.trim()
    }
}

/// Parse proto file content and return extracted property entries.
///
/// Each entry: `(enum_name, enum_index, comment_or_empty)`.
pub fn parse_css_properties_proto(content: &str) -> Vec<(String, u32, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut pending_comment = String::new();
    let mut in_enum = false;

    for line in lines {
        let trimmed = line.trim();

        // Detect start of CssProperty enum
        if trimmed.starts_with("enum CssProperty") {
            in_enum = true;
            continue;
        }

        if !in_enum {
            continue;
        }

        // Detect end of enum
        if trimmed.starts_with('}') {
            break;
        }

        // Check for comment line
        if let Some(comment) = extract_comment(line) {
            if !comment.is_empty() {
                pending_comment = comment.to_string();
            }
            continue;
        }

        // Check for enum entry
        if let Some((name, val)) = parse_enum_entry(line) {
            let comment = pending_comment.clone();
            pending_comment.clear();
            results.push((name.to_string(), val, comment));
        }
    }

    results
}

/// Build a PropertyNode from a parsed proto entry.
pub fn build_node(
    enum_name: &str,
    enum_idx: u32,
    comment: &str,
    proto_file: &str,
) -> PropertyNode {
    let name = enum_to_property_name(enum_name);
    let mut node = PropertyNode::new(&name, proto_file, enum_idx);

    if !comment.is_empty() {
        node.value_syntax = parse_value_syntax(comment).to_string();
        // Try to extract initial value from syntax if present
        // (proto comments may contain "initial: ..." patterns in extended format)
    }

    // Infer properties
    node.affects_layout = affects_layout(&name);
    node.affects_paint = affects_paint(&name);
    node.animatable = animatable(&name);

    // Inheritance inference (CSS spec knowledge)
    node.inherits = match name.as_str() {
        // Inherited properties
        "border-collapse" | "border-spacing" | "caption-side" | "color"
        | "cursor" | "direction" | "empty-cells" | "font-family"
        | "font-size" | "font-style" | "font-weight" | "font-width"
        | "font-variant" | "font-variant-ligatures" | "font-variant-position"
        | "font-variant-caps" | "font-variant-numeric" | "font-variant-east-asian"
        | "font-feature-settings" | "font-variation-settings" | "font-kerning"
        | "font-optical-sizing" | "font-palette" | "font-variant-emoji"
        | "font-synthesis" | "font-synthesis-weight" | "font-synthesis-style"
        | "font-synthesis-small-caps" | "font-synthesis-position"
        | "hanging-punctuation" | "hyphens" | "letter-spacing"
        | "line-break" | "line-height" | "list-style" | "list-style-image"
        | "list-style-position" | "list-style-type" | "orphans"
        | "overflow-wrap" | "pointer-events" | "quotes"
        | "tab-size" | "table-layout" | "text-align" | "text-align-all"
        | "text-align-last" | "text-combine-upright" | "text-indent"
        | "text-justify" | "text-orientation" | "text-transform"
        | "text-decoration" | "unicode-bidi" | "visibility"
        | "white-space" | "widows" | "word-break" | "word-spacing"
        | "writing-mode" => true,
        // Explicitly NOT inherited
        _ => false,
    };

    // Computed value inference
    node.computed_value = computed_value_for(&name);

    // Shorthand/longhand mappings
    if let Some(longhands) = shorthand_longhands(&name) {
        node.longhands = longhands.into_iter().map(String::from).collect();
    }

    node
}

/// Infer the computed value description for a property.
fn computed_value_for(name: &str) -> String {
    match name {
        "font-size" => "as specified, but absolute lengths for <absolute-size>".into(),
        "font-weight" => "the font-weight value as specified".into(),
        "line-height" => "the specified number, or the absolute length".into(),
        "opacity" => "the specified value, clamped to [0, 1]".into(),
        "color" => "the specified color".into(),
        "background-color" => "the computed color".into(),
        "width" | "height" => "as specified, but auto becomes the used value".into(),
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left"
        | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            "the percentage as specified, or the absolute length".into()
        }
        "z-index" => "as specified".into(),
        "top" | "right" | "bottom" | "left" => "the percentage as specified or the absolute length".into(),
        "transform" => "as specified".into(),
        "visibility" => "as specified".into(),
        "display" => "as specified".into(),
        "position" => "as specified".into(),
        _ => "as specified".into(),
    }
}

/// Populate a PropertyGraph from proto file contents.
pub fn populate_from_proto_contents(
    graph: &mut crate::graph::PropertyGraph,
    proto_content: &str,
    proto_file: &str,
) {
    let entries = parse_css_properties_proto(proto_content);
    for (enum_name, enum_idx, comment) in entries {
        let node = build_node(&enum_name, enum_idx, &comment, proto_file);
        graph.insert(node);
    }

    // Second pass: set up shorthand_for reverse references
    let shorthands: BTreeMap<String, Vec<String>> = graph
        .iter()
        .filter(|n| !n.longhands.is_empty())
        .map(|n| (n.name.clone(), n.longhands.clone()))
        .collect();

    for (shorthand, longhands) in &shorthands {
        for longhand in longhands {
            if let Some(node) = graph.nodes.get_mut(longhand) {
                node.shorthand_for = Some(shorthand.clone());
            }
        }
    }

    // Set up dependency edges
    populate_dependencies(graph);
}

/// Populate dependency edges between properties.
fn populate_dependencies(graph: &mut crate::graph::PropertyGraph) {
    let deps: &[(&str, &[&str])] = &[
        ("line-height", &["font-size"]),
        ("font-size-adjust", &["font-size"]),
        ("border-color", &["border-top-color"]),
        ("border-style", &["border-top-style"]),
        ("border-width", &["border-top-width"]),
        ("margin", &["margin-top"]),
        ("padding", &["padding-top"]),
        ("background", &["background-color"]),
        ("font", &["font-size", "font-family"]),
        ("flex", &["flex-grow"]),
        ("animation", &["animation-name", "animation-duration"]),
        ("transition", &["transition-property", "transition-duration"]),
        ("transform-origin", &["transform"]),
        ("grid", &["grid-template-columns"]),
        ("overflow", &["overflow-x"]),
    ];

    for (property, dependencies) in deps {
        if let Some(node) = graph.nodes.get_mut(*property) {
            for dep in dependencies.iter() {
                if !node.depends_on.iter().any(|d| d.as_str() == *dep) {
                    node.depends_on.push(dep.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PropertyGraph;

    const SAMPLE_PROTO: &str = r#"
enum CssProperty {
  CSS_PROPERTY_UNSPECIFIED = 0;
  // animation-duration: <time [0s,∞]>#
  ANIMATION_DURATION = 2;
  // background-color: <color>
  BACKGROUND_COLOR = 10;
  // font-size: <absolute-size> | <relative-size> | <length-percentage [0,∞]> | math
  FONT_SIZE = 60;
  // margin-top: <length-percentage> | auto
  MARGIN_TOP = 36;
  DISPLAY = 44;
  // visibility: <integer>
  VISIBILITY = 45;
}
"#;

    #[test]
    fn test_parse_enum_entries() {
        let entries = parse_css_properties_proto(SAMPLE_PROTO);
        assert_eq!(entries.len(), 6);
        assert_eq!(entries[0].0, "ANIMATION_DURATION");
        assert_eq!(entries[0].1, 2);
        assert!(entries[0].2.contains("animation-duration"));
        assert_eq!(entries[1].0, "BACKGROUND_COLOR");
        assert_eq!(entries[1].1, 10);
        assert_eq!(entries[2].0, "FONT_SIZE");
        assert_eq!(entries[2].1, 60);
        assert_eq!(entries[3].0, "MARGIN_TOP");
        assert_eq!(entries[3].1, 36);
        assert_eq!(entries[4].0, "DISPLAY");
        assert_eq!(entries[4].1, 44);
        assert!(entries[4].2.is_empty()); // no comment for DISPLAY
        assert_eq!(entries[5].0, "VISIBILITY");
        assert_eq!(entries[5].1, 45);
    }

    #[test]
    fn test_enum_to_property_name() {
        assert_eq!(enum_to_property_name("FONT_SIZE"), "font-size");
        assert_eq!(enum_to_property_name("BACKGROUND_COLOR"), "background-color");
        assert_eq!(enum_to_property_name("ANIMATION_DURATION"), "animation-duration");
        assert_eq!(enum_to_property_name("MARGIN_TOP"), "margin-top");
        assert_eq!(enum_to_property_name("DISPLAY"), "display");
    }

    #[test]
    fn test_parse_value_syntax() {
        assert_eq!(
            parse_value_syntax("font-size: <absolute-size> | <relative-size> | <length>"),
            "<absolute-size> | <relative-size> | <length>"
        );
        assert_eq!(parse_value_syntax("DISPLAY"), "DISPLAY");
    }

    #[test]
    fn test_build_node() {
        let node = build_node("FONT_SIZE", 60, "font-size: <absolute-size> | <relative-size> | <length-percentage [0,∞]> | math", "css/css_properties.proto");
        assert_eq!(node.name, "font-size");
        assert_eq!(node.proto_enum_idx, 60);
        assert!(node.value_syntax.contains("absolute-size"));
        assert!(node.inherits);
        assert!(node.affects_layout); // font-size affects layout
        assert!(node.animatable);
    }

    #[test]
    fn test_build_node_margin() {
        let node = build_node("MARGIN_TOP", 36, "margin-top: <length-percentage> | auto", "css/css_properties.proto");
        assert_eq!(node.name, "margin-top");
        assert!(node.affects_layout);
        assert!(!node.inherits);
    }

    #[test]
    fn test_build_node_background_color() {
        let node = build_node("BACKGROUND_COLOR", 10, "background-color: <color>", "css/css_properties.proto");
        assert_eq!(node.name, "background-color");
        assert!(node.affects_paint);
        assert!(!node.inherits);
        assert!(node.animatable);
    }

    #[test]
    fn test_shorthand_longhands() {
        let lh = shorthand_longhands("background").unwrap();
        assert_eq!(lh.len(), 8);
        assert!(lh.contains(&"background-color"));

        let lh = shorthand_longhands("font").unwrap();
        assert!(lh.contains(&"font-family"));
        assert!(lh.contains(&"font-size"));
        assert!(lh.contains(&"font-style"));
        assert!(lh.contains(&"font-weight"));
        assert!(lh.contains(&"line-height"));
    }

    #[test]
    fn test_populate_from_proto() {
        let mut graph = PropertyGraph::new();
        populate_from_proto_contents(&mut graph, SAMPLE_PROTO, "css/css_properties.proto");

        assert_eq!(graph.len(), 6);
        assert!(graph.get("font-size").is_some());
        assert!(graph.get("background-color").is_some());

        let fs = graph.get("font-size").unwrap();
        assert!(fs.inherits);
        assert!(fs.affects_layout);
        assert!(fs.animatable);
    }
}
