//! CSS styling layer — stylesheet parsing + computed style resolution.
//!
//! Converts raw CSS declaration strings into typed [`edgerun_layout::render_object::ComputedStyle`]
//! values using the value parser.

mod css_parser;
mod computed_style;

pub use css_parser::{
    parse_css, Stylesheet, Rule, Declarations, KNOWN_PROPERTIES, KNOWN_KEYWORDS,
    is_valid_property,
};
pub use computed_style::{compute_style, default_style};
