// Extract CSS Selectors Level 4: pseudo-classes, pseudo-elements, combinators, attr selectors.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"regexp"
)

type PseudoClass struct {
	Name        string `json:"name"`
	Arguments   string `json:"arguments"`
	Description string `json:"description"`
}

type PseudoElement struct {
	Name string `json:"name"`
}

type Combinator struct {
	Name        string `json:"name"`
	Syntax      string `json:"syntax"`
	Char        string `json:"char"`
	Description string `json:"description"`
}

type AttrOp struct {
	Operator   string `json:"operator,omitempty"`
	Name       string `json:"name"`
	Description string `json:"description"`
	Modifier   string `json:"modifier,omitempty"`
}

type Specificity struct {
	Component   string `json:"component"`
	Name        string `json:"name"`
	Description string `json:"description"`
}

type SelectorCatalog struct {
	Spec             string       `json:"spec"`
	SpecDate         string       `json:"spec_date"`
	TotalPseudoClasses int        `json:"total_pseudo_classes"`
	TotalPseudoElements int       `json:"total_pseudo_elements"`
	TotalCombinators int          `json:"total_combinators"`
	TotalAttrOps     int          `json:"total_attr_ops"`
	PseudoClasses    []PseudoClass `json:"pseudo_classes"`
	PseudoElements   []PseudoElement `json:"pseudo_elements"`
	Combinators      []Combinator `json:"combinators"`
	AttrOps          []AttrOp     `json:"attribute_selector_operators"`
	Specificity      []Specificity `json:"specificity"`
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: extract-selectors-spec <selectors-spec.md>")
		os.Exit(1)
	}
	data, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	md := string(data)

	// Extract pseudo-classes from spec
	pcs := make(map[string]string)
	pcRe := regexp.MustCompile(`\[§\s*[\d.]+\s+(:[\w-]+)(?:\(([^)]*)\))?\s*pseudo-class\]`)
	for _, m := range pcRe.FindAllStringSubmatch(md, -1) {
		name, args := m[1], m[2]
		if _, ok := pcs[name]; !ok {
			pcs[name] = args
		}
	}

	// Supplement with full CSS Selectors 4 list
	knownPCs := map[string]string{
		":is": "forgiving selector list", ":where": "forgiving selector list",
		":not": "forgiving selector list", ":has": "forgiving selector list",
		":hover": "", ":active": "", ":focus": "", ":focus-within": "",
		":focus-visible": "", ":visited": "", ":link": "", ":target": "",
		":target-within": "", ":lang": "language list", ":dir": "ltr or rtl",
		":any-link": "", ":local-link": "", ":scope": "", ":defined": "",
		":checked": "", ":indeterminate": "", ":default": "", ":valid": "",
		":invalid": "", ":in-range": "", ":out-of-range": "", ":required": "",
		":optional": "", ":blank": "", ":placeholder-shown": "", ":read-only": "",
		":read-write": "", ":disabled": "", ":enabled": "", ":playing": "",
		":paused": "", ":current": "selector list", ":past": "selector list",
		":future": "selector list", ":host": "selector list",
		":host-context": "selector list", ":popover-open": "", ":modal": "",
		":fullscreen": "", ":picture-in-picture": "", ":autofill": "",
		":user-invalid": "",
	}
	for name, args := range knownPCs {
		if _, ok := pcs[name]; !ok {
			pcs[name] = args
		}
	}

	var pseudoClasses []PseudoClass
	for name, args := range pcs {
		pseudoClasses = append(pseudoClasses, PseudoClass{Name: name, Arguments: args})
	}

	// Pseudo-elements
	peNames := make(map[string]bool)
	peRe := regexp.MustCompile(`::[\w][\w-]*`)
	for _, m := range peRe.FindAllString(md, -1) {
		peNames[m] = true
	}
	knownPEs := []string{"::before", "::after", "::first-line", "::first-letter",
		"::marker", "::placeholder", "::selection", "::backdrop", "::file-selector-button",
		"::column", "::page", "::slotted", "::shadow", "::lang", "::part"}
	for _, pe := range knownPEs {
		peNames[pe] = true
	}
	var pseudoElements []PseudoElement
	for name := range peNames {
		pseudoElements = append(pseudoElements, PseudoElement{Name: name})
	}

	combinators := []Combinator{
		{Name: "descendant", Syntax: " ", Char: "", Description: "E F matches F descendant of E"},
		{Name: "child", Syntax: ">", Char: ">", Description: "E > F matches F direct child of E"},
		{Name: "next_sibling", Syntax: "+", Char: "+", Description: "E + F matches F next sibling of E"},
		{Name: "subsequent_sibling", Syntax: "~", Char: "~", Description: "E ~ F matches F subsequent sibling of E"},
		{Name: "column", Syntax: "||", Char: "||", Description: "E || F matches F in column E"},
	}

	attrOps := []AttrOp{
		{Name: "exists", Description: "[attr] -- element has attribute"},
		{Operator: "=", Name: "equals", Description: `[attr="val"] -- exact match`},
		{Operator: "~=", Name: "contains_word", Description: `[attr~="val"] -- whitespace-separated word`},
		{Operator: "|=", Name: "prefix_hyphen", Description: `[attr|="val"] -- equals or prefix with hyphen`},
		{Operator: "^=", Name: "prefix", Description: `[attr^="val"] -- starts with`},
		{Operator: "$=", Name: "suffix", Description: `[attr$="val"] -- ends with`},
		{Operator: "*=", Name: "contains", Description: `[attr*="val"] -- contains substring`},
		{Modifier: "i", Name: "case_insensitive", Description: "Case-insensitive match"},
		{Modifier: "s", Name: "case_sensitive", Description: "Case-sensitive match"},
	}

	specificity := []Specificity{
		{Component: "A", Name: "id_selectors", Description: "Count of ID selectors"},
		{Component: "B", Name: "class_attr_pseudo", Description: "Classes, attributes, pseudo-classes"},
		{Component: "C", Name: "type_pseudo_element", Description: "Type selectors and pseudo-elements"},
	}

	catalog := SelectorCatalog{
		Spec: "Selectors Level 4", SpecDate: "2026-04-10",
		TotalPseudoClasses: len(pseudoClasses), TotalPseudoElements: len(pseudoElements),
		TotalCombinators: len(combinators), TotalAttrOps: len(attrOps),
		PseudoClasses: pseudoClasses, PseudoElements: pseudoElements,
		Combinators: combinators, AttrOps: attrOps, Specificity: specificity,
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
