//! HTML/CSS → RenderObject + pixel rendering pipeline.
//! DO NOT EDIT. Regenerate with: scripts/generate_html_parser.py
extern crate alloc;

pub mod html_parser;
pub mod tokenizer;
pub mod tree_builder;
pub mod entity_decoder;
pub mod css_parser;
pub mod computed_style;
pub mod layout_builder;
pub mod render;

pub use render::render;
pub use render::count_nodes;
