// Extract WHATWG HTML element definitions from spec markdown → JSON catalog.
//
// Usage: go run ./cmd/extract-html-spec docs/html_spec.md > scripts/html_element_catalog.json
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"strings"
)

type AttrDef struct {
	Name        string `json:"name"`
	Description string `json:"description"`
}

type Element struct {
	TagName                string     `json:"tag_name"`
	Categories             []string   `json:"categories"`
	Contexts               string     `json:"contexts"`
	ContentModel           string     `json:"content_model"`
	TagOmission            string     `json:"tag_omission"`
	HasGlobalAttributes    bool       `json:"has_global_attributes"`
	ElementSpecificAttrs   []AttrDef  `json:"element_specific_attributes"`
	DOMInterface           string     `json:"dom_interface"`
	IDL                    string     `json:"idl"`
	Represents             string     `json:"represents"`
}

type Catalog struct {
	Spec          string    `json:"spec"`
	SpecDate      string    `json:"spec_date"`
	TotalElements int       `json:"total_elements"`
	Elements      []Element `json:"elements"`
}

func extractElements(md string) []Element {
	re := regexp.MustCompile(`(?m)^####\s+.*?<span[^>]*?dfn-type="element">` + "`" + `(\w+)` + "`" + `</span>`)
	matches := re.FindAllStringSubmatchIndex(md, -1)

	var elements []Element
	for i, m := range matches {
		tagName := md[m[2]:m[3]]
		start := m[0]
		end := len(md)
		if i+1 < len(matches) {
			end = matches[i+1][0]
		}
		block := md[start:end]

		el := Element{TagName: tagName}

		// Categories
		catRe := regexp.MustCompile(`(?s)<a href="#concept-element-categories".*?</a>:\s*\n(.*?)(?=\n\n|\n<a href="#concept-element)`)
		if catM := catRe.FindStringSubmatch(block); len(catM) > 1 {
			raw := strings.TrimSpace(catM[1])
			if !strings.EqualFold(raw, "none.") {
				cats := regexp.MustCompile(`<a href="#([^"]+?)-content[^"]*"[^>]*>([^<]+)</a>`).FindAllStringSubmatch(raw, -1)
				for _, c := range cats {
					el.Categories = append(el.Categories, c[1])
				}
				if len(cats) == 0 {
					el.Categories = []string{raw}
				}
			}
		}

		// Contexts
		el.Contexts = extractField(block, "concept-element-contexts")
		// Content model
		el.ContentModel = extractField(block, "concept-element-content-model")
		// Tag omission
		el.TagOmission = extractField(block, "concept-element-tag-omission")

		// Attributes section
		attrsBlock := extractAttrsBlock(block)
		if attrsBlock != "" {
			if strings.Contains(attrsBlock, "global-attributes") ||
				strings.Contains(attrsBlock, "Global\nattributes") ||
				strings.Contains(attrsBlock, "Global attributes") {
				el.HasGlobalAttributes = true
			}
			attrRe := regexp.MustCompile(`\[` + "`" + `(\w+)` + "`" + `\]\([^)]*\)\s*--\s*(.+?)(?:\n\n|\n[` + "`" + `|\n<a href="#concept-element-accessibility|\n<a href="#concept-element-dom)`)
			for _, am := range attrRe.FindAllStringSubmatch(attrsBlock, -1) {
				desc := stripHTML(strings.TrimSpace(am[2]))
				desc = regexp.MustCompile(`\s+`).ReplaceAllString(desc, " ")
				el.ElementSpecificAttrs = append(el.ElementSpecificAttrs, AttrDef{Name: am[1], Description: desc})
			}
		}

		// DOM interface (IDL block)
		idlRe := regexp.MustCompile(`(?s)<a href="#concept-element-dom".*?</a>:\s*\n` + "```" + ` idl\n(.*?)` + "```")
		if idlM := idlRe.FindStringSubmatch(block); len(idlM) > 1 {
			el.IDL = strings.TrimSpace(idlM[1])
			ifaceM := regexp.MustCompile(`(?:(?:\[Exposed=Window\]\s*\n)?interface\s+(\w+))`).FindStringSubmatch(el.IDL)
			if len(ifaceM) > 1 {
				el.DOMInterface = ifaceM[1]
			}
		}

		elements = append(elements, el)
	}
	return elements
}

func extractField(block, conceptID string) string {
	re := regexp.MustCompile(`(?s)<a href="#` + conceptID + `".*?</a>:\s*\n(.*?)(?=\n\n|\n<a href="#concept-element)`)
	if m := re.FindStringSubmatch(block); len(m) > 1 {
		return stripHTML(strings.TrimSpace(m[1]))
	}
	return ""
}

func extractAttrsBlock(block string) string {
	re := regexp.MustCompile(`(?s)concept-element-attributes.*?Content attributes[</a>:]*\s*\n(.*?)(?:<a href="#concept-element-accessibility-considerations"|<a href="#concept-element-dom"|^####)`)
	if m := re.FindStringSubmatch(block); len(m) > 1 {
		return m[1]
	}
	return ""
}

func stripHTML(s string) string {
	s = regexp.MustCompile(`<[^>]+>`).ReplaceAllString(s, "")
	s = strings.ReplaceAll(s, "&amp;", "&")
	s = strings.ReplaceAll(s, "&lt;", "<")
	s = strings.ReplaceAll(s, "&gt;", ">")
	s = strings.ReplaceAll(s, "&#39;", "'")
	s = strings.ReplaceAll(s, "&quot;", `"`)
	s = strings.ReplaceAll(s, "&nbsp;", " ")
	s = regexp.MustCompile(`\s+`).ReplaceAllString(s, " ")
	return strings.TrimSpace(s)
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: extract-html-spec <path-to-html-spec.md>")
		os.Exit(1)
	}
	data, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	md := string(data)
	elements := extractElements(md)
	catalog := Catalog{
		Spec:          "WHATWG HTML Living Standard",
		SpecDate:      "2026-04-07",
		TotalElements: len(elements),
		Elements:      elements,
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
