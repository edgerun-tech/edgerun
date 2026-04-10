//! CSS Fonts types — from CSS Fonts Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_batch4.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontDisplay {
    /// Browser default
    Auto,
    /// Invisible period then swap
    Block,
    /// Fallback then swap when ready
    Swap,
    /// Brief invisible then swap
    Fallback,
    /// May skip font entirely
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontVariantLigatures {
    Normal,
    None,
    CommonLigatures,
    NoCommonLigatures,
    DiscretionaryLigatures,
    NoDiscretionaryLigatures,
    HistoricalLigatures,
    NoHistoricalLigatures,
    Contextual,
    NoContextual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontVariantCaps {
    Normal,
    SmallCaps,
    AllSmallCaps,
    PetiteCaps,
    AllPetiteCaps,
    Unicase,
    TitlingCaps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    /// 100
    Thin,
    /// 200
    ExtraLight,
    /// 300
    Light,
    /// 400
    Normal,
    /// 500
    Medium,
    /// 600
    SemiBold,
    /// 700
    Bold,
    /// 800
    ExtraBold,
    /// 900
    Black,
}

#[derive(Debug, Clone)]
pub struct FontFaceDescriptor {
    pub src: Vec<String>,
    pub font_family: String,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub font_stretch: Option<String>,
    pub font_display: Option<FontDisplay>,
    pub unicode_range: Option<String>,
    pub font_variant: Option<String>,
    pub font_feature_settings: Option<String>,
}