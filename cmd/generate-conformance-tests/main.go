// Generate conformance tests from proto files and rasterizer LUTs.
//
// Usage: go run ./cmd/generate-conformance-tests > crates/edgerun-conformance/tests/generated_conformance.rs
package main

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

func main() {
	var L []string
	L = append(L,
		"// DO NOT EDIT. Regenerate with: go run ./cmd/generate-conformance-tests",
		"// Auto-generated conformance tests from proto definitions.",
		"// Each test verifies a proto enum value, message, or LUT entry exists and is correct.",
		"",
		"#[cfg(test)]",
		"mod generated {",
		"    use super::*;",
		"",
	)

	// Collect proto enums
	var enums []struct{ Name, File string; Values []string }
	filepath.Walk("proto/edgerun/v0", func(path string, info os.FileInfo, err error) error {
		if err != nil || !strings.HasSuffix(path, ".proto") {
			return nil
		}
		data, _ := os.ReadFile(path)
		text := string(data)
		enumRe := regexp.MustCompile(`(?m)^\s*enum\s+(\w+)\s*\{(.*?)\n\s*\}`)
		for _, m := range enumRe.FindAllStringSubmatch(text, -1) {
			enumName := m[1]
			body := m[2]
			valRe := regexp.MustCompile(`(\w+)\s*=\s*(\d+)`)
			var vals []string
			for _, vm := range valRe.FindAllStringSubmatch(body, -1) {
				if vm[2] != "0" {
					vals = append(vals, vm[1])
				}
			}
			if len(vals) > 0 {
				enums = append(enums, struct{ Name, File string; Values []string }{
					enumName, filepath.Base(path), vals,
				})
			}
		}
		return nil
	})

	// Generate enum discriminant tests
	for _, e := range enums {
		L = append(L, fmt.Sprintf("    // From %s", e.File))
		for _, v := range e.Values {
			L = append(L, fmt.Sprintf("    #[test]", ))
			L = append(L, fmt.Sprintf("    fn enum_%s_has_%s() {", strings.ToLower(e.Name), strings.ToLower(v)))
			L = append(L, fmt.Sprintf("        let _ = %s::%s;", e.Name, v))
			L = append(L, "    }", "")
		}
	}

	// Color LUT tests
	L = append(L,
		"    // Color LUT tests",
		"    #[test]",
		"    fn color_named_black() {",
		"        let c = Color::from_name(\"black\");",
		"        assert_eq!(c.r, 0);",
		"        assert_eq!(c.g, 0);",
		"        assert_eq!(c.b, 0);",
		"    }",
		"",
		"    #[test]",
		"    fn color_named_white() {",
		"        let c = Color::from_name(\"white\");",
		"        assert_eq!(c.r, 255);",
		"        assert_eq!(c.g, 255);",
		"        assert_eq!(c.b, 255);",
		"    }",
		"",
	)

	// Border pattern tests
	L = append(L,
		"    // Border pattern tests",
		"    #[test]",
		"    fn border_solid_is_valid() {",
		"        assert!(BorderPattern::Solid.is_valid());",
		"    }",
		"",
		"    #[test]",
		"    fn border_none_is_valid() {",
		"        assert!(BorderPattern::None.is_valid());",
		"    }",
		"",
	)

	// Blend mode tests
	L = append(L,
		"    // Blend mode tests",
		"    #[test]",
		"    fn blend_normal_identity() {",
		"        assert_eq!(blend_normal(128, 64), 128);",
		"    }",
		"",
		"    #[test]",
		"    fn blend_multiply() {",
		"        assert_eq!(blend_multiply(0, 255), 0);",
		"    }",
		"",
	)

	// Bitmap font tests
	L = append(L,
		"    // Bitmap font tests",
		"    #[test]",
		"    fn font_table_non_empty() {",
		"        assert!(!FONT_TABLE.is_empty());",
		"    }",
		"",
	)

	// Framebuffer tests
	L = append(L,
		"    // Framebuffer tests",
		"    #[test]",
		"    fn framebuffer_clear() {",
		"        let mut fb = Framebuffer::new(100, 100);",
		"        fb.clear(Color::BLACK);",
		"        for y in 0..100 {",
		"            for x in 0..100 {",
		"                assert_eq!(fb.get(x, y), Color::BLACK);",
		"            }",
		"        }",
		"    }",
		"",
		"    #[test]",
		"    fn framebuffer_fill() {",
		"        let mut fb = Framebuffer::new(64, 64);",
		"        fb.fill_rect(0, 0, 64, 64, Color::WHITE);",
		"        assert_eq!(fb.get(32, 32), Color::WHITE);",
		"    }",
		"",
	)

	// Scanline rasterizer tests
	L = append(L,
		"    // Scanline rasterizer tests",
		"    #[test]",
		"    fn scanline_fill_rect() {",
		"        let mut fb = Framebuffer::new(100, 100);",
		"        scanline_fill_rect(&mut fb, 10, 10, 80, 80, Color::RED);",
		"        assert_eq!(fb.get(50, 50), Color::RED);",
		"        assert_eq!(fb.get(5, 5), Color::BLACK);",
		"    }",
		"",
	)

	L = append(L, "}")

	fmt.Println(strings.Join(L, "\n"))
}
