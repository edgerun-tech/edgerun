package spec

import (
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
	"google.golang.org/protobuf/proto"

	pb "edgerunrefcore/proto/go/spec"
)

func Cmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "spec",
		Short: "Extract web platform specs into proto catalogs",
		Long: `Parse W3C/WHATWG specification markdown files and produce
proto binary catalogs for code generation.

Usage:
  edgerun spec extract css <specs_dir>     > catalog.pb
  edgerun spec extract dom <file.md>       > catalog.pb
  edgerun spec extract html <file.md>      > catalog.pb
  edgerun spec extract ecmascript <file.md> > catalog.pb
  edgerun spec extract selectors <file.md>  > catalog.pb
  edgerun spec extract encoding             > catalog.pb
  edgerun spec extract fetch                > catalog.pb
  edgerun spec extract url                  > catalog.pb
  edgerun spec extract uievents             > catalog.pb
  edgerun spec extract webidl               > catalog.pb
  edgerun spec extract flexbox              > catalog.pb
  edgerun spec extract grid                 > catalog.pb

All commands output proto binary to stdout.`,
	}

	cssCmd := &cobra.Command{
		Use:   "css <specs_dir>",
		Short: "Extract CSS properties from W3C spec markdown",
		Args:  cobra.ExactArgs(1),
		RunE:  runCSS,
	}
	cmd.AddCommand(cssCmd)

	domCmd := &cobra.Command{
		Use:   "dom <file.md>",
		Short: "Extract DOM interfaces from WHATWG spec markdown",
		Args:  cobra.ExactArgs(1),
		RunE:  runDOM,
	}
	cmd.AddCommand(domCmd)

	htmlCmd := &cobra.Command{
		Use:   "html <file.md>",
		Short: "Extract HTML element definitions from WHATWG spec",
		Args:  cobra.ExactArgs(1),
		RunE:  runHTML,
	}
	cmd.AddCommand(htmlCmd)

	ecmaCmd := &cobra.Command{
		Use:   "ecmascript <file.md>",
		Short: "Extract ECMAScript built-in objects and abstract ops",
		Args:  cobra.ExactArgs(1),
		RunE:  runECMAScript,
	}
	cmd.AddCommand(ecmaCmd)

	selCmd := &cobra.Command{
		Use:   "selectors <file.md>",
		Short: "Extract Selectors Level 4 pseudo-classes and combinators",
		Args:  cobra.ExactArgs(1),
		RunE:  runSelectors,
	}
	cmd.AddCommand(selCmd)

	encCmd := &cobra.Command{
		Use:   "encoding",
		Short: "Extract encoding spec (hardcoded table)",
		RunE:  runEncoding,
	}
	cmd.AddCommand(encCmd)

	fetchCmd := &cobra.Command{
		Use:   "fetch",
		Short: "Extract fetch spec enums (hardcoded)",
		RunE:  runFetch,
	}
	cmd.AddCommand(fetchCmd)

	urlCmd := &cobra.Command{
		Use:   "url",
		Short: "Extract URL spec states and properties",
		RunE:  runURL,
	}
	cmd.AddCommand(urlCmd)

	uiCmd := &cobra.Command{
		Use:   "uievents",
		Short: "Extract UI Events key codes and button enums",
		RunE:  runUIEvents,
	}
	cmd.AddCommand(uiCmd)

	webidlCmd := &cobra.Command{
		Use:   "webidl",
		Short: "Extract Web IDL type system",
		RunE:  runWebIDL,
	}
	cmd.AddCommand(webidlCmd)

	flexCmd := &cobra.Command{
		Use:   "flexbox",
		Short: "Extract CSS Flexbox properties",
		RunE:  runFlexbox,
	}
	cmd.AddCommand(flexCmd)

	gridCmd := &cobra.Command{
		Use:   "grid",
		Short: "Extract CSS Grid properties",
		RunE:  runGrid,
	}
	cmd.AddCommand(gridCmd)

	mediaCmd := &cobra.Command{
		Use:   "media",
		Short: "Extract CSS media query types",
		RunE:  runMedia,
	}
	cmd.AddCommand(mediaCmd)

	syntaxCmd := &cobra.Command{
		Use:   "syntax",
		Short: "Extract CSS syntax token types and states",
		RunE:  runSyntax,
	}
	cmd.AddCommand(syntaxCmd)

	colorCmd := &cobra.Command{
		Use:   "color",
		Short: "Extract CSS color definitions",
		RunE:  runColor,
	}
	cmd.AddCommand(colorCmd)

	return cmd
}

// writeProto outputs the catalog as proto binary to stdout.
func writeProto(c *pb.Catalog) error {
	data, err := proto.Marshal(c)
	if err != nil {
		return fmt.Errorf("marshal catalog: %w", err)
	}
	_, err = os.Stdout.Write(data)
	return err
}

// ---- CSS Extractor ----

func runCSS(cmd *cobra.Command, args []string) error {
	specsDir := args[0]
	entries, err := os.ReadDir(specsDir)
	if err != nil {
		return fmt.Errorf("read specs dir: %w", err)
	}

	cat := &pb.Catalog{
		SpecName: "W3C CSS Specifications",
		SpecDate: "2026-04-10",
	}

	for _, e := range entries {
		if !strings.HasSuffix(e.Name(), ".md") {
			continue
		}
		if !strings.HasPrefix(e.Name(), "drafts.csswg.org_css") &&
			!strings.HasPrefix(e.Name(), "www.w3.org_TR_css") &&
			!strings.HasPrefix(e.Name(), "drafts.csswg.org_mediaqueries") {
			continue
		}
		extractCSSFile(specsDir+"/"+e.Name(), cat)
	}

	dedupCSS(cat)
	return writeProto(cat)
}

func extractCSSFile(path string, cat *pb.Catalog) {
	data, err := os.ReadFile(path)
	if err != nil {
		return
	}
	text := string(data)
	base := extractBase(path)

	// Extract properties: Name: ... dfn-type="property"...>name</span>
	for _, prop := range regexFindCSSProperties(text, base) {
		cat.CssProperties = append(cat.CssProperties, prop)
	}
	// Extract value types from css-values files
	if strings.Contains(base, "css-values") {
		for _, vt := range regexFindCSSValueTypes(text) {
			cat.CssValueTypes = append(cat.CssValueTypes, vt)
		}
	}
	// Extract at-rules
	for _, ar := range regexFindCSSAtRules(text) {
		cat.CssAtRules = append(cat.CssAtRules, ar)
	}
}

func extractBase(path string) string {
	parts := strings.Split(path, "/")
	return parts[len(parts)-1]
}

func regexFindCSSProperties(text, source string) []*pb.CssProperty {
	results := regexFindAll(`Name:\s*\n+(?:.*?\n)*?dfn-type="property"[^>]*>([^<]+)</span>`, text)
	var props []*pb.CssProperty
	for _, name := range results {
		name = strings.TrimSpace(name)
		if name == "" {
			continue
		}
		prop := &pb.CssProperty{Name: name, SourceFile: source}
		// Extract sub-fields
		for _, field := range []string{"value", "initial", "applies_to", "inherited", "percentages", "computed_value", "animation_type"} {
			val := regexExtractField(text, field)
			switch field {
			case "value":
				prop.Value = stripHTML(val)
			case "initial":
				prop.Initial = stripHTML(val)
			case "applies_to":
				prop.AppliesTo = stripHTML(val)
			case "inherited":
				prop.Inherited = strings.EqualFold(stripHTML(val), "yes")
			case "percentages":
				prop.Percentages = stripHTML(val)
			case "computed_value":
				prop.ComputedValue = stripHTML(val)
			case "animation_type":
				prop.AnimationType = stripHTML(val)
			}
		}
		props = append(props, prop)
	}
	return props
}

func regexFindCSSValueTypes(text string) []*pb.CssValueType {
	results := regexFindAll(`dfn-type="(value|type)"[^>]*>(?:&lt;)?([^<]+?)(?:&gt;)?</span>`, text)
	var vts []*pb.CssValueType
	seen := make(map[string]bool)
	// Results come in pairs: group1=kind, group2=name
	for i := 0; i+1 < len(results); i += 2 {
		kind := strings.TrimSpace(results[i])
		name := strings.TrimSpace(results[i+1])
		name = strings.Trim(name, "<>")
		if name == "" || len(name) < 2 {
			continue
		}
		key := strings.ToLower(name)
		if seen[key] {
			continue
		}
		seen[key] = true
		vts = append(vts, &pb.CssValueType{Name: name, Kind: kind})
	}
	return vts
}

func regexFindCSSAtRules(text string) []*pb.CssAtRule {
	results := regexFindAll(`dfn-type="at-rule"[^>]*>@?([^<]+)</span>`, text)
	var rules []*pb.CssAtRule
	seen := make(map[string]bool)
	for _, name := range results {
		name = strings.TrimSpace(strings.TrimLeft(name, "@"))
		if name == "" || seen[strings.ToLower(name)] {
			continue
		}
		seen[strings.ToLower(name)] = true
		rules = append(rules, &pb.CssAtRule{Name: name})
	}
	return rules
}

func regexExtractField(text, field string) string {
	capField := strings.Title(strings.ReplaceAll(field, "_", " "))
	// Match [Field:] or Field: followed by content until next field or end
	pattern := `(?:\[?` + capField + `:\]?\s*(?:\([^)]*\))?\s*\n+)(.*?)(?=\n+\[?[A-Z][\w ]+:\]?\s*(?:\([^)]*\))?\s*\n|\z)`
	results := regexFindAll(pattern, text)
	if len(results) > 0 {
		return strings.TrimSpace(results[0])
	}
	return ""
}

func dedupCSS(cat *pb.Catalog) {
	seen := make(map[string]bool)
	var props []*pb.CssProperty
	for _, p := range cat.CssProperties {
		k := strings.ToLower(p.Name)
		if !seen[k] {
			seen[k] = true
			props = append(props, p)
		}
	}
	cat.CssProperties = props
}

func stripHTML(s string) string {
	// Remove HTML tags, collapse whitespace, decode entities
	s = regexReplace(`<[^>]+>`, s, "")
	s = strings.ReplaceAll(s, "&lt;", "<")
	s = strings.ReplaceAll(s, "&gt;", ">")
	s = strings.ReplaceAll(s, "&amp;", "&")
	s = strings.ReplaceAll(s, "&#39;", "'")
	s = strings.ReplaceAll(s, "&quot;", `"`)
	s = strings.ReplaceAll(s, "&nbsp;", " ")
	s = regexReplace(`\s+`, s, " ")
	s = regexReplace(`\[\d+\]`, s, "")
	return strings.TrimSpace(s)
}
