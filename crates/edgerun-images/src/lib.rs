//! CSS Images types — from CSS Images Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientType {
    /// Gradient along a line
    LinearGradient,
    /// Repeating linear gradient
    RepeatingLinearGradient,
    /// Gradient radiating from center
    RadialGradient,
    /// Repeating radial gradient
    RepeatingRadialGradient,
    /// Gradient around a cone
    ConicGradient,
    /// Repeating conic gradient
    RepeatingConicGradient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeType {
    Circle,
    Ellipse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKeyword {
    ClosestSide,
    ClosestCorner,
    FarthestSide,
    FarthestCorner,
    Contain,
    Cover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideOrCorner {
    ToLeft,
    ToRight,
    ToTop,
    ToBottom,
    ToTopLeft,
    ToTopRight,
    ToBottomLeft,
    ToBottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    /// url() — external image resource
    Url,
    /// image() — image with fallback
    Image,
    /// image-set() — resolution-dependent images
    ImageSet,
    /// cross-fade() — blended images
    CrossFade,
    /// element() — live element snapshot
    Element,
    /// paint() — CSS Paint API worklet
    Paint,
    /// linear-gradient()
    LinearGradient,
    /// radial-gradient()
    RadialGradient,
    /// conic-gradient()
    ConicGradient,
}

/// A CSS image value.
#[derive(Debug, Clone, PartialEq)]
pub enum CssImage {
    Url(String),
    LinearGradient { angle: Option<String>, stops: Vec<CssColorStop> },
    RadialGradient { shape: Option<ShapeType>, size: Option<SizeKeyword>, position: Option<String>, stops: Vec<CssColorStop> },
    ConicGradient { from_angle: Option<String>, position: Option<String>, stops: Vec<CssColorStop> },
    Image(Vec<String>),
    ImageSet(Vec<ImageSetEntry>),
    CrossFade(Vec<CssImage>),
    Paint(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssColorStop { pub color: String, pub position: Option<String> }

#[derive(Debug, Clone, PartialEq)]
pub struct ImageSetEntry { pub url: String, pub resolution: Option<String>, pub media: Option<String> }