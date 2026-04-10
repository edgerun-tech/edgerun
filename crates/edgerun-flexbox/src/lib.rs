//! CSS Flexbox types — from CSS Flexible Box Layout spec.
//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform2.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    Nowrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Start,
    End,
    Left,
    Right,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
    Start,
    End,
    SelfStart,
    SelfEnd,
    FirstBaseline,
    LastBaseline,
    Normal,
}

/// Flex container properties.
#[derive(Debug, Clone)]
pub struct FlexContainer {
    pub direction: FlexDirection,
    pub wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_content: AlignItems,
    pub align_items: AlignItems,
    pub row_gap: Option<String>,
    pub column_gap: Option<String>,
}

/// Flex item properties.
#[derive(Debug, Clone)]
pub struct FlexItem {
    pub order: i32,
    pub flex_grow: f64,
    pub flex_shrink: f64,
    pub flex_basis: Option<String>,
    pub align_self: Option<AlignItems>,
}

impl Default for FlexItem {
    fn default() -> Self {
        Self { order: 0, flex_grow: 0.0, flex_shrink: 1.0, flex_basis: None, align_self: None }
    }
}