// Generate protobuf definitions for CSS properties from catalog JSON.
//
// Usage: go run ./cmd/generate-css-proto scripts/css_property_catalog.json proto/edgerun/v0/css/
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

type CSSCatalog struct {
	Properties   []CSSProp    `json:"properties"`
	ValueTypes   []CSSVT      `json:"value_types"`
	AtRules      []CSSAtRule  `json:"at_rules"`
}

type CSSProp struct {
	Name          string `json:"name"`
	Value         string `json:"value"`
	Initial       string `json:"initial"`
	AppliesTo     string `json:"applies_to"`
	Inherited     string `json:"inherited"`
	Percentages   string `json:"percentages"`
	ComputedValue string `json:"computed_value"`
	AnimationType string `json:"animation_type"`
}

type CSSVT struct {
	Name string `json:"name"`
	Kind string `json:"kind"`
}

type CSSAtRule struct {
	Name string `json:"name"`
}

func toUpperSnake(s string) string {
	s = strings.ReplaceAll(s, "-", "_")
	s = regexp.MustCompile(`([a-z])([A-Z])`).ReplaceAllString(s, `${1}_${2}`)
	return strings.ToUpper(s)
}

func genPropertiesProto(c *CSSCatalog) string {
	var L []string
	L = append(L, `syntax = "proto3";`,
		``,
		`package edgerun.v0.css.properties;`,
		``,
		`/// CSS Property Definitions -- generated from W3C CSS specifications.`,
		`/// DO NOT EDIT. Regenerate with: go run ./cmd/generate-css-proto`,
		``,
		`/// All CSS properties defined in W3C CSS specifications.`,
		fmt.Sprintf(`/// Total: %d properties.`, len(c.Properties)),
		`enum CssProperty {`,
		`  CSS_PROPERTY_UNSPECIFIED = 0;`)

	for _, p := range c.Properties {
		enum := toUpperSnake(p.Name)
		desc := strings.ReplaceAll(p.Value, "`", "")
		if desc != "" && len(desc) < 120 {
			L = append(L, fmt.Sprintf(`  // %s: %s`, p.Name, desc))
		}
		L = append(L, fmt.Sprintf(`  %s = %d;`, enum, len(L)))
	}

	L = append(L, `}`, ``,
		`/// Complete definition of a CSS property.`,
		`message CssPropertyDefinition {`,
		`  CssProperty property = 1;`,
		`  string name = 2;`,
		`  string value_syntax = 3;`,
		`  string initial_value = 4;`,
		`  string applies_to = 5;`,
		`  bool inherited = 6;`,
		`  string percentages = 7;`,
		`  string computed_value = 8;`,
		`  string animation_type = 9;`,
		`  string canonical_order = 10;`,
		`  string source_spec = 11;`,
		`}`, ``,
		`/// A CSS property-value pair (as used in a declaration).`,
		`message CssDeclaration {`,
		`  CssProperty property = 1;`,
		`  string value = 2;`,
		`  bool important = 3;`,
		`}`, ``,
		`/// Whether a property inherits by default.`,
		`enum Inheritance {`,
		`  INHERITANCE_UNSPECIFIED = 0;`,
		`  INHERITANCE_INHERITED = 1;`,
		`  INHERITANCE_NOT_INHERITED = 2;`,
		`  INHERITANCE_SPECIAL = 3;`,
		`}`, ``)

	return strings.Join(L, "\n")
}

func genValueTypesProto(c *CSSCatalog) string {
	var L []string
	L = append(L, `syntax = "proto3";`,
		``,
		`package edgerun.v0.css.value_types;`,
		``,
		`/// CSS Value Type Definitions -- generated from W3C CSS specifications.`,
		`/// DO NOT EDIT. Regenerate with: go run ./cmd/generate-css-proto`,
		``,
		`/// All CSS value types.`,
		fmt.Sprintf(`/// Total: %d value types.`, len(c.ValueTypes)),
		`enum CssValueType {`,
		`  CSS_VALUE_TYPE_UNSPECIFIED = 0;`)

	for _, vt := range c.ValueTypes {
		name := vt.Name
		if strings.HasPrefix(name, "-") || strings.HasPrefix(name, "+") {
			continue
		}
		enum := toUpperSnake(name)
		if len(enum) == 0 || !((enum[0] >= 'A' && enum[0] <= 'Z') || (enum[0] >= 'a' && enum[0] <= 'z')) {
			continue
		}
		L = append(L, fmt.Sprintf(`  %s = %d; // %s (%s)`, enum, len(L), name, vt.Kind))
	}

	L = append(L, `}`, ``,
		`/// Definition of a CSS value type.`,
		`message ValueTypeDef {`,
		`  CssValueType value_type = 1;`,
		`  string name = 2;`,
		`  string kind = 3;`,
		`}`, ``,
		`/// A CSS value of any type.`,
		`message CssValue {`,
		`  oneof value {`,
		`    string keyword = 1;`,
		`    string identifier = 2;`,
		`    string string_value = 3;`,
		`    double number_value = 4;`,
		`    double length_value = 5;`,
		`    string length_unit = 6;`,
		`    double percentage_value = 7;`,
		`    CssColor color = 8;`,
		`    CssFunction function = 9;`,
		`    CssUrl url = 10;`,
		`  }`,
		`}`, ``,
		`/// CSS color value.`,
		`message CssColor {`,
		`  oneof color {`,
		`    string named_color = 1;`,
		`    CssRgb rgb = 2;`,
		`    CssHsl hsl = 3;`,
		`    CssHwb hwb = 4;`,
		`    CssLab lab = 5;`,
		`    CssLch lch = 6;`,
		`    string oklch = 7;`,
		`    string oklab = 8;`,
		`    string current_color = 9;`,
		`    string hex = 10;`,
		`  }`,
		`}`, ``,
		`message CssRgb { double r = 1; double g = 2; double b = 3; optional double alpha = 4; }`,
		`message CssHsl { double h = 1; double s = 2; double l = 3; optional double alpha = 4; }`,
		`message CssHwb { double h = 1; double w = 2; double b = 3; optional double alpha = 4; }`,
		`message CssLab { double l = 1; double a = 2; double b = 3; optional double alpha = 4; }`,
		`message CssLch { double l = 1; double c = 2; double h = 3; optional double alpha = 4; }`,
		``,
		`/// CSS function call (e.g., calc(), var(), env()).`,
		`message CssFunction {`,
		`  string name = 1;`,
		`  repeated CssValue arguments = 2;`,
		`}`, ``,
		`/// CSS url() value.`,
		`message CssUrl { string url = 1; }`, ``,
		`/// Common CSS keywords.`,
		`enum CssKeyword {`,
		`  CSS_KEYWORD_UNSPECIFIED = 0;`,
		`  CSS_KEYWORD_AUTO = 1;`,
		`  CSS_KEYWORD_NONE = 2;`,
		`  CSS_KEYWORD_INHERIT = 3;`,
		`  CSS_KEYWORD_INITIAL = 4;`,
		`  CSS_KEYWORD_UNSET = 5;`,
		`  CSS_KEYWORD_REVERT = 6;`,
		`  CSS_KEYWORD_REVERT_LAYER = 7;`,
		`  CSS_KEYWORD_CURRENT_COLOR = 8;`,
		`}`, ``)

	return strings.Join(L, "\n")
}

func genAtRulesProto(c *CSSCatalog) string {
	var L []string
	L = append(L, `syntax = "proto3";`,
		``,
		`package edgerun.v0.css.at_rules;`,
		``,
		`import "edgerun/v0/css/css_properties.proto";`,
		``,
		`/// CSS At-Rule Definitions -- generated from W3C CSS specifications.`,
		`/// DO NOT EDIT. Regenerate with: go run ./cmd/generate-css-proto`,
		``,
		`/// All CSS at-rules.`,
		fmt.Sprintf(`/// Total: %d at-rules.`, len(c.AtRules)),
		`enum CssAtRule {`,
		`  CSS_AT_RULE_UNSPECIFIED = 0;`)

	for _, r := range c.AtRules {
		enum := toUpperSnake(r.Name)
		L = append(L, fmt.Sprintf(`  %s = %d; // @%s`, enum, len(L), r.Name))
	}

	L = append(L, `}`, ``,
		`/// A CSS at-rule.`,
		`message CssAtRuleBlock {`,
		`  CssAtRule at_rule = 1;`,
		`  string prelude = 2;`,
		`  repeated properties.CssDeclaration declarations = 3;`,
		`  repeated CssAtRuleBlock nested_rules = 4;`,
		`}`, ``)

	return strings.Join(L, "\n")
}

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Usage: generate-css-proto <catalog.json> <output_dir>")
		os.Exit(1)
	}

	data, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	var catalog CSSCatalog
	if err := json.Unmarshal(data, &catalog); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	outDir := os.Args[2]
	os.MkdirAll(outDir, 0755)

	files := map[string]string{
		"css_properties.proto": genPropertiesProto(&catalog),
		"css_value_types.proto": genValueTypesProto(&catalog),
		"css_at_rules.proto":   genAtRulesProto(&catalog),
	}

	for fn, content := range files {
		fp := filepath.Join(outDir, fn)
		if err := os.WriteFile(fp, []byte(content), 0644); err != nil {
			fmt.Fprintf(os.Stderr, "error writing %s: %v\n", fp, err)
			os.Exit(1)
		}
		fmt.Printf("Generated: %s\n", fp)
	}
	fmt.Printf("\nDone. %d proto files generated.\n", len(files))
}
