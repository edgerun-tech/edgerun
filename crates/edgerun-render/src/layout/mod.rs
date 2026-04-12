//! Layout layer — DOM+CSS → RenderObject tree → positioned geometry.
//!
//! ## Modules
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `render_object` | RenderObject enum (block, flex, grid, text, image containers) |
//! | `layout_context` | Formatting context detection and layout algorithm dispatch |
//! | `position_layout` | Two-pass block layout (measure heights, assign Y positions) |
//! | `box_model` | Margin, border, padding, content calculations |
//! | `text_layout` | Font weight resolution, word wrapping, text transforms |
//! | `color_convert` | HSL/OKLCh → sRGB color space conversions |
//! | `paint_command` | Display list commands for the rasterizer |
//! | `incremental` | Dirty subtree tracking for incremental relayout |
//! | `budget` | Predictive layout cost estimation |

pub mod render_object;
mod layout_context;
pub mod position_layout;
mod box_model;
mod text_layout;
mod color_convert;
mod paint_command;
pub mod layout_builder;
pub mod incremental;
pub mod budget;

pub use render_object::{RenderObject, ComputedStyle, FontSelection, FormattingContext, LayoutAlgorithm};
pub use position_layout::{position_tree, PositionedNode, PositionedKind};
pub use layout_builder::build_layout;
pub use incremental::{DirtySet, DomNodeRef, speedup_ratio};
pub use budget::LayoutBudget;
