//! CSS Media Queries types — from Media Queries Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec, boxed::Box};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    All,
    Print,
    Screen,
    Speech,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFeature {
    /// Viewport width
    Width,
    /// Viewport height
    Height,
    /// Width-to-height ratio
    AspectRatio,
    /// Viewport orientation
    Orientation,
    /// Device pixel density
    Resolution,
    /// Scanning method
    Scan,
    /// Grid vs bitmap device
    Grid,
    /// Update frequency
    Update,
    /// Overflow handling
    OverflowBlock,
    /// Inline overflow
    OverflowInline,
    /// Bits per color component
    Color,
    /// Entries in color lookup table
    ColorIndex,
    /// Bits per pixel in monochrome
    Monochrome,
    /// Supported color gamut
    ColorGamut,
    /// Colors inverted by OS
    InvertedColors,
    /// Pointing device accuracy
    Pointer,
    /// Hover capability
    Hover,
    /// Any pointing device
    AnyPointer,
    /// Any hover capability
    AnyHover,
    /// Reduced motion preference
    PrefersReducedMotion,
    /// Reduced transparency
    PrefersReducedTransparency,
    /// Contrast preference
    PrefersContrast,
    /// Color scheme preference
    PrefersColorScheme,
    /// Reduced data preference
    PrefersReducedData,
    /// Display mode
    DisplayMode,
    /// Scripting capability
    Scripting,
    /// Device screen width
    DeviceWidth,
    /// Device screen height
    DeviceHeight,
}

impl MediaFeature {
    pub fn name(&self) -> &'static str {
        match self {
            MediaFeature::Width => "width",
            MediaFeature::Height => "height",
            MediaFeature::AspectRatio => "aspect-ratio",
            MediaFeature::Orientation => "orientation",
            MediaFeature::Resolution => "resolution",
            MediaFeature::Scan => "scan",
            MediaFeature::Grid => "grid",
            MediaFeature::Update => "update",
            MediaFeature::OverflowBlock => "overflow-block",
            MediaFeature::OverflowInline => "overflow-inline",
            MediaFeature::Color => "color",
            MediaFeature::ColorIndex => "color-index",
            MediaFeature::Monochrome => "monochrome",
            MediaFeature::ColorGamut => "color-gamut",
            MediaFeature::InvertedColors => "inverted-colors",
            MediaFeature::Pointer => "pointer",
            MediaFeature::Hover => "hover",
            MediaFeature::AnyPointer => "any-pointer",
            MediaFeature::AnyHover => "any-hover",
            MediaFeature::PrefersReducedMotion => "prefers-reduced-motion",
            MediaFeature::PrefersReducedTransparency => "prefers-reduced-transparency",
            MediaFeature::PrefersContrast => "prefers-contrast",
            MediaFeature::PrefersColorScheme => "prefers-color-scheme",
            MediaFeature::PrefersReducedData => "prefers-reduced-data",
            MediaFeature::DisplayMode => "display-mode",
            MediaFeature::Scripting => "scripting",
            MediaFeature::DeviceWidth => "device-width",
            MediaFeature::DeviceHeight => "device-height",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaCondition {
    Feature(MediaFeature, Option<String>),
    Not(Box<MediaCondition>),
    And(Vec<MediaCondition>),
    Or(Vec<MediaCondition>),
}

#[derive(Debug, Clone)]
pub struct MediaQuery {
    pub media_type: Option<MediaType>,
    pub condition: Option<MediaCondition>,
}