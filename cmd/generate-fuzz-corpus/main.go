// Generate fuzz targets and seed corpus from proto definitions.
//
// Usage: go run ./cmd/generate-fuzz-corpus proto/edgerun/v0/ crates/edgerun-fuzz/
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Usage: generate-fuzz-corpus <proto_dir> <fuzz_dir>")
		os.Exit(1)
	}
	protoDir := os.Args[1]
	fuzzDir := os.Args[2]

	// Collect all enums from proto files
	type EnumDef struct {
		Name   string
		Values []EnumValue
		File   string
	}
	type EnumValue struct {
		Name string
		Num  int
	}

	var allEnums []EnumDef
	filepath.Walk(protoDir, func(path string, info os.FileInfo, err error) error {
		if err != nil || !strings.HasSuffix(path, ".proto") {
			return nil
		}
		data, _ := os.ReadFile(path)
		text := string(data)
		enumRe := regexp.MustCompile(`(?m)^\s*enum\s+(\w+)\s*\{(.*?)\n\s*\}`)
		for _, m := range enumRe.FindAllStringSubmatch(text, -1) {
			enumName := m[1]
			body := m[2]
			valRe := regexp.MustCompile(`(\w+)\s*=\s*(-?\d+)`)
			var vals []EnumValue
			for _, vm := range valRe.FindAllStringSubmatch(body, -1) {
				num := 0
				fmt.Sscanf(vm[2], "%d", &num)
				vals = append(vals, EnumValue{Name: vm[1], Num: num})
			}
			if len(vals) > 0 {
				allEnums = append(allEnums, EnumDef{Name: enumName, Values: vals, File: filepath.Base(path)})
			}
		}
		return nil
	})

	// Generate fuzz targets
	for _, e := range allEnums {
		fn := strings.ToLower(e.Name)
		targetPath := filepath.Join(fuzzDir, "fuzz_targets", fn+".rs")
		content := fmt.Sprintf(`// Fuzz target for %s -- generated from %s
// DO NOT EDIT. Regenerate with: go run ./cmd/generate-fuzz-corpus
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 { return; }
    let val = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    // Boundary checks
    if val == 0 { return; } // unspecified
    // Range validity check
    let valid = matches!(val, `, e.Name, e.File)

		// Build match arms for valid ranges
		for _, v := range e.Values {
			content += fmt.Sprintf("%d", v.Num)
			content += " | "
		}
		content = strings.TrimSuffix(content, " | ")
		content += `);
    assert!(valid || val < 0, "invalid %s discriminant: {}", val);
});
`
		os.MkdirAll(filepath.Dir(targetPath), 0755)
		os.WriteFile(targetPath, []byte(content), 0644)
		fmt.Printf("  fuzz_targets/%s.rs (%d seed values)\n", fn, len(e.Values))

		// Generate seed corpus
		corpusDir := filepath.Join(fuzzDir, "corpus", fn)
		os.MkdirAll(corpusDir, 0755)
		for _, v := range e.Values {
			bytes := []byte{byte(v.Num), byte(v.Num >> 8), byte(v.Num >> 16), byte(v.Num >> 24)}
			os.WriteFile(filepath.Join(corpusDir, v.Name), bytes, 0644)
		}
		// Edge cases
		edgeCases := map[string][]byte{
			"zero":      {0, 0, 0, 0},
			"min_i32":   {0, 0, 0, 0x80},
			"max_i32":   {0xff, 0xff, 0xff, 0x7f},
			"minus_one": {0xff, 0xff, 0xff, 0xff},
		}
		for name, bytes := range edgeCases {
			os.WriteFile(filepath.Join(corpusDir, name), bytes, 0644)
		}
	}

	// Specialized fuzz targets
	specialized := []struct {
		Name    string
		Content string
	}{
		{"css_value_parse", `// Fuzz target for CSS value parsing
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = edgerun_css_value_parser::parse_value(s);
    }
});`},
		{"css_color_parse", `// Fuzz target for CSS color parsing
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = edgerun_color::Color::parse(s);
    }
});`},
		{"css_border_parse", `// Fuzz target for CSS border parsing
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = edgerun_html_render::parse_border(s);
    }
});`},
	}

	for _, s := range specialized {
		targetPath := filepath.Join(fuzzDir, "fuzz_targets", s.Name+".rs")
		os.MkdirAll(filepath.Dir(targetPath), 0755)
		os.WriteFile(targetPath, []byte(s.Content), 0644)
		fmt.Printf("  fuzz_targets/%s.rs\n", s.Name)
	}

	fmt.Printf("\nGenerated %d fuzz targets\n", len(allEnums)+len(specialized))
}
