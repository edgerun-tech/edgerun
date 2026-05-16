package spec

import (
	"github.com/spf13/cobra"

	pb "edgerunrefcore/proto/go/spec"
)

// ---- Selectors Extractor ----

func runSelectors(cmd *cobra.Command, args []string) error {
	cat := &pb.Catalog{SpecName: "Selectors Level 4"}

	// Pseudo-classes
	for _, pc := range pseudoClasses {
		cat.PseudoClasses = append(cat.PseudoClasses, &pb.PseudoClassDef{
			Name:        pc.name,
			Arguments:   pc.args,
			Description: pc.desc,
		})
	}
	// Pseudo-elements
	for _, pe := range pseudoElements {
		cat.PseudoElements = append(cat.PseudoElements, &pb.PseudoElementDef{Name: pe})
	}
	// Combinators
	for _, c := range combinatorsData {
		cat.Combinators = append(cat.Combinators, &pb.CombinatorDef{
			Name: c.name, Syntax: c.syntax, Char: c.char, Description: c.desc,
		})
	}
	// Attr match ops
	for _, a := range attrOps {
		cat.AttrMatchOps = append(cat.AttrMatchOps, &pb.AttrMatchOpDef{
			Name: a.name, Operator: a.op, Description: a.desc,
		})
	}
	return writeProto(cat)
}

type pcEntry struct{ name, args, desc string }

var pseudoClasses = []pcEntry{
	{"active", "", "User action pseudo-class"},
	{"any-link", "", "Matches links"},
	{"checked", "", "User action pseudo-class"},
	{"default", "", "User action pseudo-class"},
	{"defined", "", "Custom pseudo-class"},
	{"dir", "ltr|rtl", "Directionality pseudo-class"},
	{"disabled", "", "User action pseudo-class"},
	{"empty", "", "Structural pseudo-class"},
	{"enabled", "", "User action pseudo-class"},
	{"first", "", "Page pseudo-class"},
	{"first-child", "", "Structural pseudo-class"},
	{"first-of-type", "", "Structural pseudo-class"},
	{"focus", "", "User action pseudo-class"},
	{"focus-visible", "", "User action pseudo-class"},
	{"focus-within", "", "User action pseudo-class"},
	{"fullscreen", "", "User action pseudo-class"},
	{"has", "selector-list", "Relational pseudo-class"},
	{"host", "", "Custom pseudo-class"},
	{"host-context", "selector", "Custom pseudo-class"},
	{"hover", "", "User action pseudo-class"},
	{"indeterminate", "", "User action pseudo-class"},
	{"in-range", "", "User action pseudo-class"},
	{"invalid", "", "User action pseudo-class"},
	{"is", "selector-list", "Functional pseudo-class"},
	{"lang", "language-range", "Linguistic pseudo-class"},
	{"last-child", "", "Structural pseudo-class"},
	{"last-of-type", "", "Structural pseudo-class"},
	{"link", "", "Location pseudo-class"},
	{"local-link", "", "Location pseudo-class"},
	{"not", "selector-list", "Negation pseudo-class"},
	{"nth-child", "An+B [of selector-list]", "Structural pseudo-class"},
	{"nth-col", "An+B", "Structural pseudo-class"},
	{"nth-last-child", "An+B [of selector-list]", "Structural pseudo-class"},
	{"nth-last-col", "An+B", "Structural pseudo-class"},
	{"nth-last-of-type", "An+B", "Structural pseudo-class"},
	{"nth-of-type", "An+B", "Structural pseudo-class"},
	{"only-child", "", "Structural pseudo-class"},
	{"only-of-type", "", "Structural pseudo-class"},
	{"optional", "", "User action pseudo-class"},
	{"out-of-range", "", "User action pseudo-class"},
	{"placeholder-shown", "", "User action pseudo-class"},
	{"read-only", "", "User action pseudo-class"},
	{"read-write", "", "User action pseudo-class"},
	{"required", "", "User action pseudo-class"},
	{"root", "", "Structural pseudo-class"},
	{"scope", "", "Static pseudo-class"},
	{"target", "", "Target pseudo-class"},
	{"target-within", "", "Target pseudo-class"},
	{"user-invalid", "", "User action pseudo-class"},
	{"valid", "", "User action pseudo-class"},
	{"visited", "", "Location pseudo-class"},
	{"where", "selector-list", "Functional pseudo-class"},
}

var pseudoElements = []string{
	"after", "backdrop", "before", "cue", "cue-region",
	"file-selector-button", "first-letter", "first-line",
	"grammar-error", "highlight", "marker",
	"part", "placeholder", "selection", "slotted",
	"spelling-error", "target-text",
}

type comboEntry struct{ name, syntax, char, desc string }

var combinatorsData = []comboEntry{
	{"descendant", "E F", " ", "Descendant combinator"},
	{"child", "E > F", ">", "Child combinator"},
	{"next-sibling", "E + F", "+", "Next-sibling combinator"},
	{"subsequent-sibling", "E ~ F", "~", "Subsequent-sibling combinator"},
	{"column", "E || F", "||", "Column combinator"},
}

type attrOpEntry struct{ name, op, desc string }

var attrOps = []attrOpEntry{
	{"attribute-exists", "", "Has attribute"},
	{"attribute-equals", "=", "Attribute value equals"},
	{"attribute-includes", "~=", "Attribute value includes whitespace-separated token"},
	{"attribute-dashmatch", "|=", "Attribute value equals or begins with value followed by hyphen"},
	{"attribute-prefix", "^=", "Attribute value begins with"},
	{"attribute-suffix", "$=", "Attribute value ends with"},
	{"attribute-substring", "*=", "Attribute value contains"},
	{"case-sensitive", "s", "Case-sensitive matching"},
	{"ascii-case-insensitive", "i", "ASCII case-insensitive matching"},
}
