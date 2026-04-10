//! RenderObject hierarchy — generated from CSS Display + HTML element specs.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py
extern crate alloc;
use alloc::{string::String, vec::Vec, boxed::Box};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormattingContext { Block, Inline, Flex, Grid, Table, Ruby, List, Math, None, Contents }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlgorithm { BlockFlow, InlineFlow, FlexMainAxis, FlexCrossAxis, GridTracks, TableGrid, RubyBase }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionType { Static, Relative, Absolute, Fixed, Sticky }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoType { Before, After, FirstLine, FirstLetter, Marker, Placeholder, Selection, Backdrop }

#[derive(Debug, Clone)]
pub struct FontSelection { pub family: String, pub weight: u32, pub size: f64, pub line_height: f64 }

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub formatting_context: FormattingContext,
    pub position: PositionType,
    pub opacity: f64,
    pub z_index: Option<i64>,
    pub background_color: Option<Color>,
    pub border_color: Color,
    pub border_width: f64,
    pub color: Color,
    pub font: FontSelection,
    pub has_transform: bool,
    pub will_change: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum RenderObject {
    BlockContainer { children: Vec<RenderObject>, style: ComputedStyle },
    FlexContainer { children: Vec<RenderObject>, style: ComputedStyle },
    GridContainer { children: Vec<RenderObject>, style: ComputedStyle },
    InlineContainer { children: Vec<RenderObject>, style: ComputedStyle },
    TextRun { text: String, font: FontSelection, style: ComputedStyle },
    Image { src: String, intrinsic_size: (Option<f64>, Option<f64>), style: ComputedStyle },
}
