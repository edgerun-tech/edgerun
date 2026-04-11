// Generate 4 crates: edgerun-images, edgerun-fonts, edgerun-media-queries, edgerun-css-syntax.
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

func rv(name string) string {
	parts := strings.Split(strings.ReplaceAll(name, "-", "_"), "_")
	for i, p := range parts {
		if len(p) > 0 {
			parts[i] = strings.ToUpper(p[:1]) + p[1:]
		}
	}
	return strings.Join(parts, "")
}

func w(path, content string) {
	os.MkdirAll(filepath.Dir(path), 0755)
	os.WriteFile(path, []byte(content), 0644)
	fmt.Printf("  %s\n", path)
}

func cargo(name string) string {
	return fmt.Sprintf(`[package]
name = "%s"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
`, name)
}

func genImages() map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargo("edgerun-images")
	var L []string
	L = append(L, "//! edgerun-images — CSS Images types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-batch4",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum GradientType { Linear, Radial, Conic }",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum ShapeType { Circle, Ellipse, Polygon, Inset }",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum SizeKeyword { ClosestSide, FarthestSide, ClosestCorner, FarthestCorner, Contain, Cover }",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum SideOrCorner { Top = 1, Right = 2, Bottom = 4, Left = 8 }",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum ImageType { Url, LinearGradient, RadialGradient, ConicGradient, Image, ImageSet, CrossFade, Paint }",
		"",
		"#[derive(Debug, Clone)]",
		"pub enum CssImage {",
		"    Url(String),",
		"    LinearGradient { angle: f32, stops: Vec<CssColorStop> },",
		"    RadialGradient { shape: ShapeType, size: SizeKeyword, pos: (f32, f32), stops: Vec<CssColorStop> },",
		"    ConicGradient { angle: f32, pos: (f32, f32), stops: Vec<CssColorStop> },",
		"    ImageSet(Vec<ImageSetEntry>),",
		"    CrossFade(Vec<(CssImage, f32)>),",
		"    Paint(String),",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct CssColorStop { color: (f32, f32, f32, f32), offset: f32 }",
		"#[derive(Debug, Clone)]",
		"pub struct ImageSetEntry { image: CssImage, resolution: f32 }")
	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

func genFonts() map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargo("edgerun-fonts")
	var L []string
	L = append(L, "//! edgerun-fonts — CSS Fonts types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-batch4",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum FontDisplay { Auto, Block, Swap, Optional, Fallback }",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum FontVariantLigatures { Normal, None, Common, Discretionary, Historical }",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum FontVariantCaps { Normal, Small, AllSmall, Petite, AllPetite, Unicase, TitlingCaps }",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum FontWeight {",
		"    Thin = 100, ExtraLight = 200, Light = 300, Normal = 400, Medium = 500,",
		"    SemiBold = 600, Bold = 700, ExtraBold = 800, Black = 900,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct FontFaceDescriptor {",
		"    pub family: String,",
		"    pub style: String,",
		"    pub weight: FontWeight,",
		"    pub display: FontDisplay,",
		"    pub src: Vec<String>,",
		"}")
	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

func genMediaQueries() map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargo("edgerun-media-queries")
	var L []string
	L = append(L, "//! edgerun-media-queries — CSS Media Queries types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-batch4",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum MediaType { All, Screen, Print, Speech }",
		"",
		"impl MediaType {",
		"    pub fn name(&self) -> &'static str {",
		"        match self {",
		`            MediaType::All => "all",`,
		`            MediaType::Screen => "screen",`,
		`            MediaType::Print => "print",`,
		`            MediaType::Speech => "speech",`,
		`        }`,
		`    }`,
		`}`, "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum MediaFeature {",
		"    Width, MinWidth, MaxWidth, Height, MinHeight, MaxHeight,",
		"    Orientation, AspectRatio, Color, ColorIndex,",
		"    Resolution, Scan, Grid, Update,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub enum MediaCondition {",
		"    Feature(MediaFeature, f32),",
		"    Not(Box<MediaCondition>),",
		"    And(Vec<MediaCondition>),",
		"    Or(Vec<MediaCondition>),",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct MediaQuery {",
		"    pub media_type: MediaType,",
		"    pub condition: Option<MediaCondition>,",
		"}")
	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

func genCSSSyntax() map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargo("edgerun-css-syntax")
	var L []string
	L = append(L, "//! edgerun-css-syntax — CSS Syntax types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-batch4",
		"#![cfg_attr(not(test), no_std)]",
		"extern crate alloc;",
		"use alloc::string::String;",
		"",
		"#[derive(Debug, Clone, PartialEq)]",
		"pub enum CssToken {",
		"    Ident(String),",
		"    Function(String),",
		"    AtKeyword(String),",
		"    Hash(String),",
		"    StringToken(String),",
		"    BadString,",
		"    Url(String),",
		"    BadUrl,",
		"    Delim(char),",
		"    Number(f64),",
		"    Percentage(f64),",
		"    Dimension(f64, String),",
		"    Whitespace,",
		"    Cdo, // <!--",
		"    Cdc, // -->",
		"    Colon, Semicolon, Comma,",
		"    LeftParen, RightParen,",
		"    LeftSquare, RightSquare,",
		"    LeftCurly, RightCurly,",
		"    Eof,",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum TokenizerState {",
		"    Top, HashStart, AtKeywordStart, HashRest, AtKeywordRest,",
		"    IdentStart, IdentRest, Number, Percentage, Dimension,",
		"    String, BadString, Url, BadUrl,",
		"    Comment, Cdo, Cdc,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct UnicodeRange {",
		"    pub start: u32,",
		"    pub end: u32,",
		"}", "",
		"impl UnicodeRange {",
		"    pub fn parse(s: &str) -> Option<Self> {",
		"        // Simplified: U+XXXX or U+XXXX-YYYY",
		"        if !s.starts_with(\"U+\") && !s.starts_with(\"u+\") { return None; }",
		"        let rest = &s[2..];",
		"        if let Some(dash) = rest.find('-') {",
		"            let start = u32::from_str_radix(&rest[..dash].replace('?', \"0\"), 16).ok()?;",
		"            let end = u32::from_str_radix(&rest[dash+1..].replace('?', \"F\"), 16).ok()?;",
		"            Some(Self { start, end })",
		"        } else {",
		"            let val = u32::from_str_radix(&rest.replace('?', \"0\"), 16).ok()?;",
		"            Some(Self { start: val, end: val })",
		"        }",
		"    }",
		"}")
	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

func main() {
	outDir := "crates"
	if len(os.Args) >= 2 {
		outDir = os.Args[1]
	}
	for rel, content := range genImages() {
		w(filepath.Join(outDir, "edgerun-images", rel), content)
	}
	for rel, content := range genFonts() {
		w(filepath.Join(outDir, "edgerun-fonts", rel), content)
	}
	for rel, content := range genMediaQueries() {
		w(filepath.Join(outDir, "edgerun-media-queries", rel), content)
	}
	for rel, content := range genCSSSyntax() {
		w(filepath.Join(outDir, "edgerun-css-syntax", rel), content)
	}
	fmt.Println("\nDone.")
}
