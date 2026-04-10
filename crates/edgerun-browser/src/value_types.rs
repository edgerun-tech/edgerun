
//! CSS Value Types — generated from W3C CSS specifications.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py
extern crate alloc;
use alloc::{string::String, vec::Vec};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssValueType {
    Keyword, Dimension, Number, Percentage, String, Url, Function, Color,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssValue {
    Keyword(&'static str),
    Dimension(f64, &'static str), // (value, unit)
    Number(f64),
    Percentage(f64),
    StringValue(String),
    Url(String),
    Function { name: String, args: Vec<CssValue> },
    Color { r: f64, g: f64, b: f64, a: f64 },
    List(Vec<CssValue>),
}

impl CssValue {
    pub fn value_type(&self) -> CssValueType {
        match self {
            Self::Keyword(_) => CssValueType::Keyword,
            Self::Dimension(_, _) => CssValueType::Dimension,
            Self::Number(_) => CssValueType::Number,
            Self::Percentage(_) => CssValueType::Percentage,
            Self::StringValue(_) => CssValueType::String,
            Self::Url(_) => CssValueType::Url,
            Self::Function { .. } => CssValueType::Function,
            Self::Color { .. } => CssValueType::Color,
            Self::List(_) => CssValueType::Keyword,
        }
    }
    pub fn is_inherit(&self) -> bool { matches!(self, Self::Keyword("inherit")) }
    pub fn is_initial(&self) -> bool { matches!(self, Self::Keyword("initial")) }
    pub fn is_unset(&self) -> bool { matches!(self, Self::Keyword("unset")) }
    pub fn is_auto(&self) -> bool { matches!(self, Self::Keyword("auto")) }
    pub fn is_none(&self) -> bool { matches!(self, Self::Keyword("none")) }
}