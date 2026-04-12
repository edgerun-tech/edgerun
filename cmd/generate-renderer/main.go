// Generate edgerun-render layout module (renderer/layout engine) from CSS/HTML proto data.
//
// Usage: go run ./cmd/generate-renderer crates/edgerun-render/src/layout/
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

func w(path, content string) {
	os.MkdirAll(filepath.Dir(path), 0755)
	os.WriteFile(path, []byte(content), 0644)
	fmt.Printf("  %s: %d lines\n", path, strings.Count(content, "\n"))
}

func main() {
	outDir := "crates/edgerun-layout"
	if len(os.Args) >= 2 {
		outDir = os.Args[1]
	}

	// Cargo.toml
	w(filepath.Join(outDir, "Cargo.toml"), `[package]
name = "edgerun-layout"
version = "0.1.0"
edition.workspace = true
license.workspace = true
publish = false
description = "Layout engine generated from CSS specifications"

[dependencies]
edgerun-html-render = { path = "../edgerun-html-render" }
edgerun-css-value-parser = { path = "../edgerun-css-value-parser" }
edgerun-color = { path = "../edgerun-color" }
edgerun-rasterizer = { path = "../edgerun-rasterizer" }
`)

	// lib.rs
	w(filepath.Join(outDir, "src", "lib.rs"), `//! edgerun-layout — Layout engine from CSS specifications.
//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-renderer
extern crate alloc;

pub mod render_object;
pub mod layout_context;
pub mod box_model;
pub mod paint_command;
pub mod color_convert;
pub mod text_layout;

#[derive(Debug, Clone, PartialEq)]
pub enum ComputedColor {
    Named(String),
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, f32),
    CurrentColor,
    Transparent,
}
`)

	// render_object.rs
	w(filepath.Join(outDir, "src", "render_object.rs"), `//! RenderObject — the output of layout.
//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-renderer
extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone)]
pub enum RenderObject {
    BlockContainer { children: Vec<RenderObject>, style: ComputedStyle },
    FlexContainer { children: Vec<RenderObject>, style: ComputedStyle, flex_direction: FlexDirection },
    GridContainer { children: Vec<RenderObject>, style: ComputedStyle, tracks: GridTracks },
    InlineContainer { children: Vec<RenderObject>, style: ComputedStyle },
    TextRun { text: String, style: ComputedStyle, font: FontSelection },
    Image { src: String, intrinsic_size: (u32, u32), style: ComputedStyle },
    PseudoElement { pseudo: PseudoElementKind, children: Vec<RenderObject> },
    ListItemMarker { style: ComputedStyle },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormattingContext { Block, Flex, Grid, Inline }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlgorithm { BFC, FlexLayout, GridLayout, InlineLayout }

#[derive(Debug, Clone)]
pub struct FontSelection {
    pub family: String,
    pub size: f32,
    pub weight: f32,
    pub style: FontStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub display: DisplayOuter,
    pub position: Position,
    pub background_color: ComputedColor,
    pub color: ComputedColor,
    pub font_size: f32,
    pub font_family: String,
    pub opacity: f32,
    pub z_index: Option<i32>,
}
`)

	// layout_context.rs
	w(filepath.Join(outDir, "src", "layout_context.rs"), `//! Layout context — dispatches to the correct formatting context.
//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-renderer
use crate::render_object::*;

pub fn formatting_context(display: &DisplayOuter) -> FormattingContext {
    match display {
        DisplayOuter::Block => FormattingContext::Block,
        DisplayOuter::Flex => FormattingContext::Flex,
        DisplayOuter::Grid => FormattingContext::Grid,
        DisplayOuter::Inline => FormattingContext::Inline,
    }
}

pub fn layout_algorithm(ctx: FormattingContext) -> LayoutAlgorithm {
    match ctx {
        FormattingContext::Block => LayoutAlgorithm::BFC,
        FormattingContext::Flex => LayoutAlgorithm::FlexLayout,
        FormattingContext::Grid => LayoutAlgorithm::GridLayout,
        FormattingContext::Inline => LayoutAlgorithm::InlineLayout,
    }
}

pub fn intrinsic_size(obj: &RenderObject) -> Option<(u32, u32)> {
    match obj {
        RenderObject::Image { intrinsic_size, .. } => Some(*intrinsic_size),
        _ => None,
    }
}

pub fn box_dimensions(obj: &RenderObject) -> (f32, f32, f32, f32) {
    // Returns (content_width, content_height, padding, border)
    match obj {
        RenderObject::TextRun { text, font, .. } => {
            let w = text.len() as f32 * font.size * 0.6;
            (w, font.size * 1.2, 0.0, 0.0)
        }
        _ => (0.0, 0.0, 0.0, 0.0),
    }
}
`)

	// box_model.rs
	w(filepath.Join(outDir, "src", "box_model.rs"), `//! Box model — margin, border, padding, content.
//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-renderer
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeValues {
    pub top: f32, pub right: f32, pub bottom: f32, pub left: f32,
}

impl Default for EdgeValues {
    fn default() -> Self { Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 } }
}

pub fn establishes_bfc(display: &DisplayOuter, position: &Position) -> bool {
    matches!(display, DisplayOuter::Block | DisplayOuter::Flex | DisplayOuter::Grid)
        || matches!(position, Position::Absolute | Position::Fixed)
}

pub fn overflow_clip(style: &ComputedStyle) -> bool {
    style.overflow == Overflow::Hidden
}

pub fn margin_box(content: (f32, f32), margin: &EdgeValues) -> (f32, f32) {
    (content.0 + margin.left + margin.right, content.1 + margin.top + margin.bottom)
}
`)

	// paint_command.rs
	w(filepath.Join(outDir, "src", "paint_command.rs"), `//! Paint commands — display list for the rasterizer.
//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-renderer
extern crate alloc;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone)]
pub enum PaintCommand {
    FillRect { x: f32, y: f32, w: f32, h: f32, color: ComputedColor },
    FillGradient { x: f32, y: f32, w: f32, h: f32, stops: Vec<(f32, ComputedColor)> },
    StrokeRect { x: f32, y: f32, w: f32, h: f32, color: ComputedColor, thickness: f32 },
    DrawText { x: f32, y: f32, text: String, color: ComputedColor, font: FontSelection },
    PushClip { x: f32, y: f32, w: f32, h: f32, radius: f32 },
    PushTransform { m: [f32; 6] },
    PushOpacity { alpha: f32 },
    DrawShadow { x: f32, y: f32, w: f32, h: f32, blur: f32, offset: (f32, f32), color: ComputedColor },
    PopClip,
    PopTransform,
    PopOpacity,
}

pub struct DisplayList {
    pub commands: Vec<PaintCommand>,
}

impl DisplayList {
    pub fn new() -> Self { Self { commands: Vec::new() } }
    pub fn push(&mut self, cmd: PaintCommand) { self.commands.push(cmd); }
}
`)

	// color_convert.rs
	w(filepath.Join(outDir, "src", "color_convert.rs"), `//! Color space conversions.
//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-renderer

/// HSL to sRGB conversion.
pub fn hsl_to_srgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let s = s / 100.0;
    let l = l / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 { (c, x, 0.0) }
        else if h < 120.0 { (x, c, 0.0) }
        else if h < 180.0 { (0.0, c, x) }
        else if h < 240.0 { (0.0, x, c) }
        else if h < 300.0 { (x, 0.0, c) }
        else { (c, 0.0, x) };
    ((r1 + m) as u8, (g1 + m) as u8, (b1 + m) as u8)
}

/// OKLCh to sRGB (simplified).
pub fn oklch_to_srgb(l: f32, c: f32, h: f32) -> (u8, u8, u8) {
    // Simplified conversion — full OKLCh requires matrix transforms
    let a = c * h.cos();
    let b = c * h.sin();
    let r = (l + 1.0 * a + 0.0 * b).clamp(0.0, 1.0);
    let g = (l - 0.5 * a + 0.5 * b).clamp(0.0, 1.0);
    let b = (l - 0.3 * a - 0.7 * b).clamp(0.0, 1.0);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
`)

	// text_layout.rs
	w(filepath.Join(outDir, "src", "text_layout.rs"), `//! Text layout — wrapping, transforms, font weight.
//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-renderer
extern crate alloc;
use alloc::string::String;

pub fn apply_text_transform(text: &str, transform: &str) -> String {
    match transform {
        "uppercase" => text.to_uppercase(),
        "lowercase" => text.to_lowercase(),
        "capitalize" => text.split_whitespace().map(|w| {
            let mut c = w.chars();
            match c.next() { None => String::new(), Some(f) => f.to_uppercase().collect::<String>() + c.as_str() }
        }).collect::<Vec<_>>().join(" "),
        _ => text.to_string(),
    }
}

pub fn should_break_word(word: &str, whitespace: &str) -> bool {
    matches!(whitespace, "normal" | "pre-wrap" | "pre-line")
}

pub fn is_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{00A0}')
}

pub fn resolve_font_weight(weight: &str) -> f32 {
    match weight {
        "normal" | "400" => 400.0,
        "bold" | "700" => 700.0,
        "100" => 100.0, "200" => 200.0, "300" => 300.0,
        "500" => 500.0, "600" => 600.0, "800" => 800.0, "900" => 900.0,
        v => v.parse().unwrap_or(400.0),
    }
}
`)

	fmt.Println("\nDone.")
}
