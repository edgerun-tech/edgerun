//! CSS Grid types — from CSS Grid Layout spec.
//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform2.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{boxed::Box, string::String, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridAutoFlow {
    Row,
    Column,
    Dense,
    RowDense,
    ColumnDense,
}

/// Grid track sizing function.
#[derive(Debug, Clone, PartialEq)]
pub enum TrackFunction {
    Minmax(Box<TrackSize>, Box<TrackSize>),
    FitContent(String),
    Repeat(u32, Vec<Box<TrackSize>>),
    MaxContent,
    MinContent,
    Auto,
    Flex(f64),
    Length(String),
}
/// Grid track size.
pub type TrackSize = TrackFunction;
/// Grid line placement.
#[derive(Debug, Clone, PartialEq)]
pub enum GridLine {
    Auto,
    Line(i32),
    NamedLine(String),
    Span(u32),
    NamedSpan(String, u32),
}
/// Grid item placement.
#[derive(Debug, Clone)]
pub struct GridPlacement {
    pub column_start: GridLine,
    pub column_end: GridLine,
    pub row_start: GridLine,
    pub row_end: GridLine,
}
/// Grid container properties.
#[derive(Debug, Clone)]
pub struct GridContainer {
    pub auto_flow: GridAutoFlow,
    pub auto_columns: Vec<TrackSize>,
    pub auto_rows: Vec<TrackSize>,
    pub row_gap: Option<String>,
    pub column_gap: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridAlignment {
    Start,
    End,
    Center,
    Stretch,
    Baseline,
    Normal,
    SelfStart,
    SelfEnd,
}
