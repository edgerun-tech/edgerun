// Generate 4 crates: edgerun-webidl, edgerun-flexbox, edgerun-grid, edgerun-color.
//
// Usage: go run ./cmd/generate-web-platform2 webidl.json flexbox.json grid.json color.json crates/
package main

import (
	"encoding/json"
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

func cargoToml(name string) string {
	return fmt.Sprintf(`[package]
name = "%s"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
`, name)
}

type WebIDLCatalog struct {
	PrimitiveTypes []string   `json:"primitive_types"`
	StringTypes    []string   `json:"string_types"`
	ObjectTypes    []string   `json:"object_types"`
	SpecialTypes   []string   `json:"special_types"`
	ExtendedAttrs  []struct {
		Name string `json:"name"`
	} `json:"extended_attributes"`
}

func genWebIDL(c WebIDLCatalog) map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargoToml("edgerun-webidl")
	var L []string
	L = append(L, "//! edgerun-webidl — Web IDL type system.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform2",
		"#![cfg_attr(not(test), no_std)]",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum PrimitiveType {")
	for _, t := range c.PrimitiveTypes {
		L = append(L, fmt.Sprintf("    %s,", rv(strings.ReplaceAll(t, " ", "_"))))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum StringType {")
	for _, t := range c.StringTypes {
		L = append(L, fmt.Sprintf("    %s,", t))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum ObjectType {")
	for _, t := range c.ObjectTypes {
		L = append(L, fmt.Sprintf("    %s,", rv(t)))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum SpecialType {")
	for _, t := range c.SpecialTypes {
		L = append(L, fmt.Sprintf("    %s,", rv(t)))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum ExtendedAttr {")
	for _, a := range c.ExtendedAttrs {
		L = append(L, fmt.Sprintf("    %s,", rv(a.Name)))
	}
	L = append(L, "}")
	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

type FlexCatalog struct {
	ContainerProps []struct{ Name, Values string } `json:"container_properties"`
	ChildProps     []struct{ Name, Values string } `json:"child_properties"`
}

func genFlexbox(c FlexCatalog) map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargoToml("edgerun-flexbox")

	// flex-direction enum
	var L []string
	L = append(L, "//! edgerun-flexbox — CSS Flexbox types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform2",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum FlexDirection {",
		"    Row, RowReverse, Column, ColumnReverse,",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum FlexWrap {",
		"    Nowrap, Wrap, WrapReverse,",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum JustifyContent {",
		"    FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly,",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum AlignItems {",
		"    FlexStart, FlexEnd, Center, Baseline, Stretch,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct FlexContainer {",
		"    pub direction: FlexDirection,",
		"    pub wrap: FlexWrap,",
		"    pub justify_content: JustifyContent,",
		"    pub align_items: AlignItems,",
		"    pub gap: f32,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct FlexItem {",
		"    pub order: i32,",
		"    pub grow: f32,",
		"    pub shrink: f32,",
		"    pub basis: f32,",
		"    pub align_self: AlignItems,",
		"}", "",
		"impl Default for FlexItem {",
		"    fn default() -> Self {",
		"        Self { order: 0, grow: 0.0, shrink: 1.0, basis: 0.0, align_self: AlignItems::Stretch }",
		"    }",
		"}")

	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

type GridCatalog struct {
	Properties []struct{ Name string } `json:"properties"`
}

func genGrid(c GridCatalog) map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargoToml("edgerun-grid")
	var L []string
	L = append(L, "//! edgerun-grid — CSS Grid Layout types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform2",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum GridAutoFlow {",
		"    Row, Column, Dense, RowDense, ColumnDense,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub enum TrackFunction {",
		"    Minmax(f32, f32),",
		"    FitContent(f32),",
		"    Repeat(u32, Vec<TrackFunction>),",
		"    MaxContent,",
		"    MinContent,",
		"    Auto,",
		"    Flex(f32),",
		"    Length(f32),",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub enum GridLine {",
		"    Auto,",
		"    Index(i32),",
		"    Named(String),",
		"    Span(u32),",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct GridPlacement {",
		"    pub start: GridLine,",
		"    pub end: GridLine,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct GridContainer {",
		"    pub columns: Vec<TrackFunction>,",
		"    pub rows: Vec<TrackFunction>,",
		"    pub auto_flow: GridAutoFlow,",
		"    pub auto_columns: Vec<TrackFunction>,",
		"    pub auto_rows: Vec<TrackFunction>,",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum GridAlignment {",
		"    Start, End, Center, Stretch, Baseline,",
		"}")
	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

func genColor() map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = cargoToml("edgerun-color")
	var L []string
	L = append(L, "//! edgerun-color — CSS Color types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform2",
		"#![cfg_attr(not(test), no_std)]",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum ColorSpace {",
		"    Srgb, DisplayP3, A98Rgb, ProphotoRgb, Rec2020, Xyz, Lab, Lch,",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum NamedColor {",
		"    Black, White, Red, Green, Blue, Yellow, Cyan, Magenta,",
		"    Orange, Purple, Pink, Brown, Gray, Silver,",
		"}", "",
		"impl NamedColor {",
		"    pub fn from_name(name: &str) -> Option<Self> {",
		"        match name {",
		`            "black" => Some(NamedColor::Black),`,
		`            "white" => Some(NamedColor::White),`,
		`            "red" => Some(NamedColor::Red),`,
		`            "green" => Some(NamedColor::Green),`,
		`            "blue" => Some(NamedColor::Blue),`,
		`            "yellow" => Some(NamedColor::Yellow),`,
		`            "cyan" => Some(NamedColor::Cyan),`,
		`            "magenta" => Some(NamedColor::Magenta),`,
		`            "orange" => Some(NamedColor::Orange),`,
		`            _ => None,`,
		`        }`,
		`    }`,
		`}`, "",
		"#[derive(Debug, Clone)]",
		"pub enum CssColor {",
		"    Named(NamedColor),",
		"    Rgb { r: u8, g: u8, b: u8, a: f32 },",
		"    Hsl { h: f32, s: f32, l: f32, a: f32 },",
		"    Hwb { h: f32, w: f32, b: f32, a: f32 },",
		"    Lab { l: f32, a: f32, b: f32, alpha: f32 },",
		"    Lch { l: f32, c: f32, h: f32, alpha: f32 },",
		"    Oklab { l: f32, a: f32, b: f32, alpha: f32 },",
		"    Oklch { l: f32, c: f32, h: f32, alpha: f32 },",
		"    CurrentColor,",
		"    Transparent,",
		"}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum InterpolationSpace {",
		"    Srgb, SrgbLinear, Oklab, Oklch, Hsl, Hwb, Lch, Xyz,",
		"}", "",
		"#[derive(Debug, Clone)]",
		"pub struct ColorMix {",
		"    pub color1: CssColor,",
		"    pub color2: CssColor,",
		"    pub percentage: f32,",
		"    pub space: InterpolationSpace,",
		"}")
	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

func loadJSON(path string, v any) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	return json.Unmarshal(data, v)
}

func main() {
	if len(os.Args) < 6 {
		fmt.Fprintln(os.Stderr, "Usage: generate-web-platform2 <webidl.json> <flexbox.json> <grid.json> <color.json> <out_dir>")
		os.Exit(1)
	}
	outDir := os.Args[5]

	var wc WebIDLCatalog
	loadJSON(os.Args[1], &wc)
	for rel, content := range genWebIDL(wc) {
		w(filepath.Join(outDir, "edgerun-webidl", rel), content)
	}

	var fc FlexCatalog
	loadJSON(os.Args[2], &fc)
	for rel, content := range genFlexbox(fc) {
		w(filepath.Join(outDir, "edgerun-flexbox", rel), content)
	}

	var gc GridCatalog
	loadJSON(os.Args[3], &gc)
	for rel, content := range genGrid(gc) {
		w(filepath.Join(outDir, "edgerun-grid", rel), content)
	}

	for rel, content := range genColor() {
		w(filepath.Join(outDir, "edgerun-color", rel), content)
	}

	fmt.Println("\nDone.")
}
