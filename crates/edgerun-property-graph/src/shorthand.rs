//! Shorthand → longhand mappings for CSS shorthand properties.

use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// Populate the shorthand map with known CSS shorthand definitions.
pub fn build_shorthands(map: &mut BTreeMap<String, Vec<String>>) {
    map.insert("background".into(), vec![
        "background-color".into(), "background-image".into(),
        "background-repeat".into(), "background-attachment".into(),
        "background-position".into(), "background-size".into(),
        "background-clip".into(), "background-origin".into(),
    ]);
    map.insert("border".into(), vec![
        "border-top-width".into(), "border-right-width".into(),
        "border-bottom-width".into(), "border-left-width".into(),
        "border-top-style".into(), "border-right-style".into(),
        "border-bottom-style".into(), "border-left-style".into(),
        "border-top-color".into(), "border-right-color".into(),
        "border-bottom-color".into(), "border-left-color".into(),
    ]);
    map.insert("border-top".into(), vec!["border-top-width".into(), "border-top-style".into(), "border-top-color".into()]);
    map.insert("border-right".into(), vec!["border-right-width".into(), "border-right-style".into(), "border-right-color".into()]);
    map.insert("border-bottom".into(), vec!["border-bottom-width".into(), "border-bottom-style".into(), "border-bottom-color".into()]);
    map.insert("border-left".into(), vec!["border-left-width".into(), "border-left-style".into(), "border-left-color".into()]);
    map.insert("border-width".into(), vec!["border-top-width".into(), "border-right-width".into(), "border-bottom-width".into(), "border-left-width".into()]);
    map.insert("border-style".into(), vec!["border-top-style".into(), "border-right-style".into(), "border-bottom-style".into(), "border-left-style".into()]);
    map.insert("border-color".into(), vec!["border-top-color".into(), "border-right-color".into(), "border-bottom-color".into(), "border-left-color".into()]);
    map.insert("border-radius".into(), vec!["border-top-left-radius".into(), "border-top-right-radius".into(), "border-bottom-right-radius".into(), "border-bottom-left-radius".into()]);
    map.insert("margin".into(), vec!["margin-top".into(), "margin-right".into(), "margin-bottom".into(), "margin-left".into()]);
    map.insert("padding".into(), vec!["padding-top".into(), "padding-right".into(), "padding-bottom".into(), "padding-left".into()]);
    map.insert("font".into(), vec!["font-style".into(), "font-variant".into(), "font-weight".into(), "font-size".into(), "line-height".into(), "font-family".into()]);
    map.insert("list-style".into(), vec!["list-style-type".into(), "list-style-position".into(), "list-style-image".into()]);
    map.insert("outline".into(), vec!["outline-color".into(), "outline-style".into(), "outline-offset".into()]);
    map.insert("animation".into(), vec!["animation-name".into(), "animation-duration".into(), "animation-timing-function".into(), "animation-iteration-count".into(), "animation-direction".into(), "animation-play-state".into(), "animation-delay".into(), "animation-fill-mode".into()]);
    map.insert("transition".into(), vec!["transition-property".into(), "transition-duration".into(), "transition-timing-function".into(), "transition-delay".into()]);
    map.insert("flex".into(), vec!["flex-grow".into(), "flex-shrink".into(), "flex-basis".into()]);
    map.insert("flex-flow".into(), vec!["flex-direction".into(), "flex-wrap".into()]);
    map.insert("text-decoration".into(), vec!["text-decoration-line".into(), "text-decoration-style".into(), "text-decoration-color".into()]);
    map.insert("inset".into(), vec!["top".into(), "right".into(), "bottom".into(), "left".into()]);
}
