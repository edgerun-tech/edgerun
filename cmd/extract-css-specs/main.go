// Extract CSS property definitions from W3C/WHATWG CSS spec markdown files.
//
// Usage: go run ./cmd/extract-css-specs docs/w3c_specs/ > scripts/css_property_catalog.json
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

type CSSProperty struct {
	Name            string `json:"name"`
	Value           string `json:"value"`
	Initial         string `json:"initial"`
	AppliesTo       string `json:"applies_to"`
	Inherited       string `json:"inherited"`
	Percentages     string `json:"percentages"`
	ComputedValue   string `json:"computed_value"`
	AnimationType   string `json:"animation_type"`
	CanonicalOrder  string `json:"canonical_order"`
	SourceFile      string `json:"source_file"`
}

type CSSValueType struct {
	Name string `json:"name"`
	Kind string `json:"kind"`
}

type CSSAtRule struct {
	Name string `json:"name"`
}

type CSSCatalog struct {
	Spec             string         `json:"spec"`
	SpecDate         string         `json:"spec_date"`
	TotalProperties  int            `json:"total_properties"`
	TotalValueTypes  int            `json:"total_value_types"`
	TotalAtRules     int            `json:"total_at_rules"`
	Properties       []CSSProperty  `json:"properties"`
	ValueTypes       []CSSValueType `json:"value_types"`
	AtRules          []CSSAtRule    `json:"at_rules"`
}

func stripHTML(s string) string {
	s = regexp.MustCompile(`<[^>]+>`).ReplaceAllString(s, "")
	s = strings.ReplaceAll(s, "&lt;", "<")
	s = strings.ReplaceAll(s, "&gt;", ">")
	s = strings.ReplaceAll(s, "&amp;", "&")
	s = strings.ReplaceAll(s, "&#39;", "'")
	s = strings.ReplaceAll(s, "&quot;", `"`)
	s = strings.ReplaceAll(s, "&nbsp;", " ")
	s = regexp.MustCompile(`\s+`).ReplaceAllString(s, " ")
	s = regexp.MustCompile(`\[\d+\]`).ReplaceAllString(s, "")
	return strings.TrimSpace(s)
}

func extractPropertiesFromMD(text, filename string) []CSSProperty {
	re := regexp.MustCompile(`(?m)^Name:\s*\n+(?:.*?\n)*?dfn-type="property"[^>]*>([^<]+)</span>`)
	matches := re.FindAllStringSubmatchIndex(text, -1)

	var props []CSSProperty
	for i, m := range matches {
		name := strings.TrimSpace(text[m[2]:m[3]])
		start := m[0]
		end := len(text)
		if i+1 < len(matches) {
			end = matches[i+1][0]
		}
		block := text[start:end]

		p := CSSProperty{Name: name, SourceFile: filename}
		for _, field := range []string{"value", "initial", "applies_to", "inherited", "percentages", "computed_value", "animation_type", "canonical_order"} {
			fieldTitle := strings.Title(strings.ReplaceAll(field, "_", " "))
			fieldRe := regexp.MustCompile(`(?s)(?:\[?` + regexp.QuoteMeta(fieldTitle) + `:\]?\s*(?:\([^)]*\))?\s*\n+)(.*?)(?=\n+\[?[A-Z][\w ]+:\]?\s*(?:\([^)]*\))?\s*\n|\z)`)
			if fm := fieldRe.FindStringSubmatch(block); len(fm) > 1 {
				v := strings.TrimSpace(fm[1])
				if v != "" {
					switch field {
					case "value":
						p.Value = stripHTML(v)
					case "initial":
						p.Initial = stripHTML(v)
					case "applies_to":
						p.AppliesTo = stripHTML(v)
					case "inherited":
						p.Inherited = stripHTML(v)
					case "percentages":
						p.Percentages = stripHTML(v)
					case "computed_value":
						p.ComputedValue = stripHTML(v)
					case "animation_type":
						p.AnimationType = stripHTML(v)
					case "canonical_order":
						p.CanonicalOrder = stripHTML(v)
					}
				}
			}
		}
		props = append(props, p)
	}
	return props
}

func extractValueTypes(text string) []CSSValueType {
	re := regexp.MustCompile(`dfn-type="(value|type)"[^>]*>(?:&lt;)?([^<]+?)(?:&gt;)?</span>`)
	matches := re.FindAllStringSubmatch(text, -1)
	seen := make(map[string]bool)
	var vts []CSSValueType
	for _, m := range matches {
		name := strings.TrimSpace(strings.Trim(m[2], "<>"))
		if name == "" || strings.Contains(strings.ToLower(name), "css") || name == "the" || name == "an" || name == "a" {
			continue
		}
		key := strings.ToLower(name)
		if seen[key] {
			continue
		}
		seen[key] = true
		vts = append(vts, CSSValueType{Name: name, Kind: m[1]})
	}
	return vts
}

func extractAtRules(text string) []CSSAtRule {
	re := regexp.MustCompile(`dfn-type="at-rule"[^>]*>@?([^<]+)</span>`)
	matches := re.FindAllStringSubmatch(text, -1)
	seen := make(map[string]bool)
	var rules []CSSAtRule
	for _, m := range matches {
		name := strings.TrimSpace(strings.TrimLeft(m[1], "@"))
		key := strings.ToLower(name)
		if seen[key] {
			continue
		}
		seen[key] = true
		rules = append(rules, CSSAtRule{Name: name})
	}
	return rules
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: extract-css-specs <specs_dir>")
		os.Exit(1)
	}
	specsDir := os.Args[1]

	entries, err := os.ReadDir(specsDir)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	var cssFiles []string
	for _, e := range entries {
		n := e.Name()
		if strings.HasSuffix(n, ".md") && (strings.HasPrefix(n, "drafts.csswg.org_css") ||
			strings.HasPrefix(n, "www.w3.org_TR_css") ||
			strings.HasPrefix(n, "drafts.csswg.org_mediaqueries")) {
			cssFiles = append(cssFiles, n)
		}
	}
	sort.Strings(cssFiles)

	var allProps []CSSProperty
	var allVTs []CSSValueType
	var allAtRules []CSSAtRule

	for _, fn := range cssFiles {
		fp := filepath.Join(specsDir, fn)
		data, err := os.ReadFile(fp)
		if err != nil {
			continue
		}
		text := string(data)
		allProps = append(allProps, extractPropertiesFromMD(text, fn)...)
		if strings.Contains(fn, "css-values") {
			allVTs = append(allVTs, extractValueTypes(text)...)
		}
		allAtRules = append(allAtRules, extractAtRules(text)...)
	}

	// Deduplicate properties
	seenP := make(map[string]bool)
	var uniqueProps []CSSProperty
	for _, p := range allProps {
		if !seenP[strings.ToLower(p.Name)] {
			seenP[strings.ToLower(p.Name)] = true
			uniqueProps = append(uniqueProps, p)
		}
	}

	catalog := CSSCatalog{
		Spec:             "W3C CSS Specifications",
		SpecDate:         "2026-04-10",
		TotalProperties:  len(uniqueProps),
		TotalValueTypes:  len(allVTs),
		TotalAtRules:     len(allAtRules),
		Properties:       uniqueProps,
		ValueTypes:       allVTs,
		AtRules:          allAtRules,
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
