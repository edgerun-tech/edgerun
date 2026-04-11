// Generate 4 web platform crates: edgerun-fetch, edgerun-url, edgerun-encoding, edgerun-uievents.
//
// Usage: go run ./cmd/generate-web-platform fetch.json url.json encoding.json uievents.json crates/
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

type FetchCatalog struct {
	RequestInit        []FetchField `json:"request_init"`
	ResponseProperties []FetchField `json:"response_properties"`
	Enums              []FetchEnum  `json:"enums"`
}
type FetchField struct {
	Name string `json:"name"`
	Type string `json:"type"`
}
type FetchEnum struct {
	Name     string   `json:"name"`
	Variants []string `json:"variants"`
}

func genFetch(c FetchCatalog) map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = `[package]
name = "edgerun-fetch"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
`
	var L []string
	L = append(L, "//! edgerun-fetch — Fetch Living Standard types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform",
		"#![cfg_attr(not(test), no_std)]",
		"extern crate alloc;")

	for _, e := range c.Enums {
		L = append(L, "",
			fmt.Sprintf("#[derive(Debug, Clone, Copy, PartialEq, Eq)]"),
			fmt.Sprintf("pub enum %s {", rv(e.Name)))
		for _, v := range e.Variants {
			L = append(L, fmt.Sprintf("    %s,", rv(v)))
		}
		L = append(L, "}")
	}

	L = append(L, "",
		"#[derive(Debug, Clone)]",
		"pub struct RequestInit {")
	for _, fi := range c.RequestInit {
		L = append(L, fmt.Sprintf("    pub %s: %s,", fi.Name, fi.Type))
	}
	L = append(L, "}")

	L = append(L, "",
		"#[derive(Debug, Clone)]",
		"pub struct Response {")
	for _, fi := range c.ResponseProperties {
		L = append(L, fmt.Sprintf("    pub %s: %s,", fi.Name, fi.Type))
	}
	L = append(L, "}")

	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

type URLCatalog struct {
	URLParserStates []string `json:"url_parser_states"`
	SpecialSchemes  []string `json:"special_schemes"`
}

func genURL(c URLCatalog) map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = `[package]
name = "edgerun-url"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
`
	var L []string
	L = append(L, "//! edgerun-url — URL Living Standard types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform",
		"#![cfg_attr(not(test), no_std)]",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum UrlParserState {")
	for _, s := range c.URLParserStates {
		L = append(L, fmt.Sprintf("    %s,", s))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum SpecialScheme {")
	for _, s := range c.SpecialSchemes {
		L = append(L, fmt.Sprintf("    %s,", rv(s)))
	}
	L = append(L, "}", "",
		"impl SpecialScheme {",
		"    pub fn default_port(&self) -> Option<u16> {",
		"        match self {")
	ports := map[string]int{"ftp": 21, "http": 80, "https": 443, "ws": 80, "wss": 443}
	for _, s := range c.SpecialSchemes {
		if p, ok := ports[strings.ToLower(s)]; ok {
			L = append(L, fmt.Sprintf(`            SpecialScheme::%s => Some(%d),`, rv(s), p))
		} else {
			L = append(L, fmt.Sprintf(`            SpecialScheme::%s => None,`, rv(s)))
		}
	}
	L = append(L, `        }`,
		`    }`,
		`}`,
		"",
		"#[derive(Debug, Clone)]",
		"pub struct Url {",
		"    pub scheme: String,",
		"    pub username: String,",
		"    pub password: String,",
		"    pub host: Option<String>,",
		"    pub port: Option<u16>,",
		"    pub path: Vec<String>,",
		"    pub query: Option<String>,",
		"    pub fragment: Option<String>,",
		"}")

	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

type EncodingCatalog struct {
	Encodings []EncodingDef `json:"encodings"`
	BOMTable  []BOMDef      `json:"bom_table"`
}
type EncodingDef struct {
	Name   string   `json:"name"`
	Labels []string `json:"labels"`
}
type BOMDef struct {
	Encoding string `json:"encoding"`
	Bytes    []int  `json:"bytes"`
}

func genEncoding(c EncodingCatalog) map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = `[package]
name = "edgerun-encoding"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
`
	var L []string
	L = append(L, "//! edgerun-encoding — Encoding Living Standard types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform",
		"#![cfg_attr(not(test), no_std)]",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum Encoding {")
	for _, e := range c.Encodings {
		L = append(L, fmt.Sprintf("    %s,", rv(e.Name)))
	}
	L = append(L, "}", "",
		"impl Encoding {",
		"    pub fn from_label(label: &str) -> Option<Self> {",
		"        let l = label.to_lowercase();")
	for _, e := range c.Encodings {
		if e.Name == "replacement" {
			continue
		}
		allLabels := append([]string{strings.ToLower(e.Name)}, e.Labels...)
		for _, lb := range allLabels {
			L = append(L, fmt.Sprintf(`        if l == "%s" { return Some(Encoding::%s); }`, lb, rv(e.Name)))
		}
	}
	L = append(L, `        None`,
		`    }`,
		`}`, "",
		"pub const BOM_TABLE: &[(Encoding, &[u8])] = &[")
	for _, b := range c.BOMTable {
		bytes := ""
		for _, by := range b.Bytes {
			bytes += fmt.Sprintf("0x%02X, ", by)
		}
		L = append(L, fmt.Sprintf(`    (Encoding::%s, &[%s]),`, rv(b.Encoding), bytes))
	}
	L = append(L, "];", "",
		"pub fn detect_bom(data: &[u8]) -> Option<(Encoding, usize)> {",
		"    for &(enc, bom) in BOM_TABLE {",
		"        if data.len() >= bom.len() && &data[..bom.len()] == bom {",
		"            return Some((enc, bom.len()));",
		"        }",
		"    }",
		"    None",
		"}")

	f["src/lib.rs"] = strings.Join(L, "\n")
	return f
}

type UICatalog struct {
	KeyboardKeys    []string `json:"keyboard_keys"`
	MouseButtons    []struct {
		Name  string `json:"name"`
		Value int    `json:"value"`
	} `json:"mouse_buttons"`
	WheelDeltaModes []struct {
		Name string `json:"name"`
	} `json:"wheel_delta_modes"`
}

func genUI(c UICatalog) map[string]string {
	f := make(map[string]string)
	f["Cargo.toml"] = `[package]
name = "edgerun-uievents"
version = "0.1.0"
edition.workspace = true
publish = false

[dependencies]
`
	var L []string
	L = append(L, "//! edgerun-uievents — UI Events types.",
		"//! DO NOT EDIT. Regenerate with: go run ./cmd/generate-web-platform",
		"#![cfg_attr(not(test), no_std)]",
		"",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum Key {")
	for _, k := range c.KeyboardKeys {
		if len(k) == 1 {
			L = append(L, fmt.Sprintf("    Char('%s'),", k))
		} else {
			L = append(L, fmt.Sprintf("    %s,", rv(k)))
		}
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum MouseButton {")
	for _, b := range c.MouseButtons {
		L = append(L, fmt.Sprintf("    %s = %d,", rv(b.Name), b.Value))
	}
	L = append(L, "}", "",
		"#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
		"pub enum DeltaMode {")
	for _, m := range c.WheelDeltaModes {
		L = append(L, fmt.Sprintf("    %s,", rv(m.Name)))
	}
	L = append(L, "}")

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
		fmt.Fprintln(os.Stderr, "Usage: generate-web-platform <fetch.json> <url.json> <encoding.json> <uievents.json> <out_dir>")
		os.Exit(1)
	}
	outDir := os.Args[5]

	var fetchC FetchCatalog
	loadJSON(os.Args[1], &fetchC)
	for rel, content := range genFetch(fetchC) {
		w(filepath.Join(outDir, "edgerun-fetch", rel), content)
	}

	var urlC URLCatalog
	loadJSON(os.Args[2], &urlC)
	for rel, content := range genURL(urlC) {
		w(filepath.Join(outDir, "edgerun-url", rel), content)
	}

	var encC EncodingCatalog
	loadJSON(os.Args[3], &encC)
	for rel, content := range genEncoding(encC) {
		w(filepath.Join(outDir, "edgerun-encoding", rel), content)
	}

	var uiC UICatalog
	loadJSON(os.Args[4], &uiC)
	for rel, content := range genUI(uiC) {
		w(filepath.Join(outDir, "edgerun-uievents", rel), content)
	}

	fmt.Println("\nDone.")
}
