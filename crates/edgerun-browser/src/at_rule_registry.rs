//! CSS At-Rule Registry — generated from W3C CSS specifications.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py
extern crate alloc;
use alloc::vec::Vec;
use crate::property_registry::CssPropertyId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtRuleId {
    Unspecified = 0,
    Keyframes = 1,
    Import = 2,
    FontFeatureValues = 3,
    FontPaletteValues = 4,
    Page = 5,
    Charset = 6,
    Media = 7,
}
impl AtRuleId {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "keyframes" => Some(AtRuleId::Keyframes),
            "import" => Some(AtRuleId::Import),
            "font-feature-values" => Some(AtRuleId::FontFeatureValues),
            "font-palette-values" => Some(AtRuleId::FontPaletteValues),
            "page" => Some(AtRuleId::Page),
            "charset" => Some(AtRuleId::Charset),
            "media" => Some(AtRuleId::Media),
            _ => None,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            AtRuleId::Keyframes => "@keyframes",
            AtRuleId::Import => "@import",
            AtRuleId::FontFeatureValues => "@font-feature-values",
            AtRuleId::FontPaletteValues => "@font-palette-values",
            AtRuleId::Page => "@page",
            AtRuleId::Charset => "@charset",
            AtRuleId::Media => "@media",
            AtRuleId::Unspecified => unreachable!(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum AtRulePrelude {
    Keyframes,
    Import,
    FontFeatureValues,
    FontPaletteValues,
    Page,
    Charset,
    Media,
}
#[derive(Debug, Clone)]
pub struct AtRuleBlock {
    pub at_rule: AtRuleId,
    pub declarations: Vec<(CssPropertyId, crate::value_types::CssValue)>,
}