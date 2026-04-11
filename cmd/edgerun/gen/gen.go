package gen

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"text/template"

	"github.com/spf13/cobra"
	"google.golang.org/protobuf/proto"

	pb "edgerunrefcore/proto/go/spec"
)

func Cmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "gen",
		Short: "Generate Rust crates from proto catalogs",
	}

	cmd.AddCommand(&cobra.Command{
		Use:   "selectors <catalog.pb> <out-selectors> <out-dom>",
		Short: "Generate edgerun-selectors and edgerun-dom crates",
		Args:  cobra.ExactArgs(3),
		RunE:  genSelectors,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "web-platform <catalog.pb> <out-dir>",
		Short: "Generate edgerun-fetch, edgerun-url, edgerun-encoding, edgerun-uievents",
		Args:  cobra.ExactArgs(2),
		RunE:  genWebPlatform,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "browser <catalog.pb> <out-dir>",
		Short: "Generate edgerun-browser crate",
		Args:  cobra.ExactArgs(2),
		RunE:  genBrowser,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "batch4 <catalog.pb> <out-dir>",
		Short: "Generate edgerun-images, fonts, media-queries, css-syntax",
		Args:  cobra.ExactArgs(2),
		RunE:  genBatch4,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "renderer <catalog.pb> <out-dir>",
		Short: "Generate edgerun-layout crate",
		Args:  cobra.ExactArgs(2),
		RunE:  genRenderer,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "css-proto <catalog.pb> <proto-out-dir>",
		Short: "Generate CSS proto files from catalog",
		Args:  cobra.ExactArgs(2),
		RunE:  genCSSProto,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "ecmascript-proto <catalog.pb> <proto-out-dir>",
		Short: "Generate ECMAScript proto files from catalog",
		Args:  cobra.ExactArgs(2),
		RunE:  genECMAScriptProto,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "web-platform2 <catalog.pb>",
		Short: "Generate edgerun-webidl, flexbox, grid, color crates",
		Args:  cobra.ExactArgs(2),
		RunE:  genWebPlatform2,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "conformance-tests <catalog.pb>",
		Short: "Generate conformance test crate",
		Args:  cobra.ExactArgs(1),
		RunE:  genConformanceTests,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "fuzz-corpus <catalog.pb>",
		Short: "Generate fuzz targets and corpus",
		Args:  cobra.ExactArgs(1),
		RunE:  genFuzzCorpus,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "wgsl <catalog.pb> <out-file>",
		Short: "Generate WGSL shaders",
		Args:  cobra.ExactArgs(2),
		RunE:  genWGSL,
	})

	cmd.AddCommand(&cobra.Command{
		Use:   "dashboard <proto-dir> <conformance-rs> <out-html>",
		Short: "Generate conformance dashboard HTML",
		Args:  cobra.ExactArgs(3),
		RunE:  genDashboard,
	})

	return cmd
}

func loadCatalog(path string) (*pb.Catalog, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read catalog: %w", err)
	}
	cat := &pb.Catalog{}
	if err := proto.Unmarshal(data, cat); err != nil {
		return nil, fmt.Errorf("unmarshal catalog: %w", err)
	}
	return cat, nil
}

func writeCrateFile(dir, name, content string) error {
	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(dir, name), []byte(content), 0644)
}

func genSelectors(cmd *cobra.Command, args []string) error {
	cat, err := loadCatalog(args[0])
	if err != nil {
		return err
	}

	outSelectors := args[1]
	outDOM := args[2]

	// Generate edgerun-selectors
	tmpls := map[string]string{
		"Cargo.toml": selectorsCargoTOML,
		"src/lib.rs": selectorsLibRS,
		"src/pseudo_classes.rs": genPseudoClasses(cat),
		"src/pseudo_elements.rs": genPseudoElements(cat),
		"src/combinators.rs":     genCombinators(cat),
		"src/attr_selectors.rs":  genAttrSelectors(cat),
		"src/specificity.rs":     specificityRS,
	}
	for name, content := range tmpls {
		if err := writeCrateFile(filepath.Join(outSelectors, filepath.Dir(name)), filepath.Base(name), content); err != nil {
			return err
		}
	}

	// Generate edgerun-dom
	domTmpls := map[string]string{
		"Cargo.toml":    domCargoTOML,
		"src/lib.rs":    domLibRS,
		"src/interfaces.rs": genInterfaces(cat),
		"src/events.rs":     genEvents(cat),
	}
	for name, content := range domTmpls {
		if err := writeCrateFile(filepath.Join(outDOM, filepath.Dir(name)), filepath.Base(name), content); err != nil {
			return err
		}
	}

	fmt.Fprintf(cmd.OutOrStdout(), "Generated %s (%d selectors) and %s (%d interfaces)\n",
		outSelectors, len(cat.PseudoClasses)+len(cat.PseudoElements),
		outDOM, len(cat.DomInterfaces))
	return nil
}

func genPseudoClasses(cat *pb.Catalog) string {
	t := template.Must(template.New("pc").Parse(`// Auto-generated from spec catalog.
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoClass {
	{{- range .PseudoClasses }}
    {{ .Name | toRustIdent }},
	{{- end }}
}

impl FromStr for PseudoClass {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
		{{- range .PseudoClasses }}
            "{{ .Name }}" => Ok(PseudoClass::{{ .Name | toRustIdent }}),
		{{- end }}
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PseudoClassSelector {
    Simple(PseudoClass),
    Functional(PseudoClass, String),
}

impl PseudoClassSelector {
    pub fn from_name(name: &str, args: Option<&str>) -> Option<Self> {
        if let Ok(pc) = name.parse::<PseudoClass>() {
            if let Some(a) = args {
                return Some(PseudoClassSelector::Functional(pc, a.to_string()));
            }
            return Some(PseudoClassSelector::Simple(pc));
        }
        None
    }
}
`))
	var buf strings.Builder
	t.Execute(&buf, cat)
	return buf.String()
}

func genPseudoElements(cat *pb.Catalog) string {
	t := template.Must(template.New("pe").Funcs(template.FuncMap{"toRustIdent": toRustIdent}).Parse(`// Auto-generated from spec catalog.
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoElement {
	{{- range .PseudoElements }}
    {{ .Name | toRustIdent }},
	{{- end }}
}

impl PseudoElement {
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
		{{- range .PseudoElements }}
            "{{ .Name }}" => Some(PseudoElement::{{ .Name | toRustIdent }}),
		{{- end }}
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
		{{- range .PseudoElements }}
            PseudoElement::{{ .Name | toRustIdent }} => "{{ .Name }}",
		{{- end }}
        }
    }
}
`))
	var buf strings.Builder
	t.Execute(&buf, cat)
	return buf.String()
}

func genCombinators(cat *pb.Catalog) string {
	lines := "// Auto-generated from spec catalog.\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Combinator {\n"
	for _, c := range cat.Combinators {
		name := toRustIdent(c.Name)
		lines += fmt.Sprintf("    %s,\n", name)
	}
	lines += "}\n\nimpl Combinator {\n    pub fn from_char(c: char) -> Option<Self> {\n        match c {\n"
	for _, c := range cat.Combinators {
		if c.Char != "" {
			name := toRustIdent(c.Name)
			lines += fmt.Sprintf("            '%s' => Some(Combinator::%s),\n", c.Char, name)
		}
	}
	lines += "            _ => None,\n        }\n    }\n}\n"
	return lines
}

func genAttrSelectors(cat *pb.Catalog) string {
	lines := "// Auto-generated from spec catalog.\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum AttrMatchOp {\n"
	for _, a := range cat.AttrMatchOps {
		name := toRustIdent(a.Name)
		lines += fmt.Sprintf("    %s,\n", name)
	}
	lines += "}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum CaseSensitivity {\n    CaseSensitive,\n    AsciiCaseInsensitive,\n}\n"
	return lines
}

func genInterfaces(cat *pb.Catalog) string {
	t := template.Must(template.New("iface").Funcs(template.FuncMap{"toRustIdent": toRustIdent}).Parse(`// Auto-generated from spec catalog.
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceId {
	{{- range .DomInterfaces }}
    {{ .Name }},
	{{- end }}
}

pub trait Node {
    fn node_type(&self) -> NodeType;
    fn owner_document(&self) -> Option<&dyn Document>;
    fn parent_node(&self) -> Option<&dyn Node>;
    fn child_nodes(&self) -> &[Box<dyn Node>];
}

pub trait Element: Node {
    fn tag_name(&self) -> &str;
    fn get_attribute(&self, name: &str) -> Option<&str>;
    fn set_attribute(&mut self, name: &str, value: &str);
    fn remove_attribute(&mut self, name: &str);
}

{{ range .DomInterfaces }}
// {{ .Name }}{{ if .Parent }} : {{ .Parent }}{{ end }}
pub trait {{ .Name }}: Node {
    {{- range .Attributes }}
    fn {{ .Name }}(&self) -> {{ .Type }};
    {{- end }}
    {{- range .Methods }}
    fn {{ .Name }}(&self{{ range .Parameters }}, {{ .Name }}: {{ .Type }}{{ end }}) -> {{ .ReturnType }};
    {{- end }}
}
{{ end }}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Element,
    Text,
    Comment,
    Document,
    DocumentFragment,
    DocumentType,
    ProcessingInstruction,
    Attr,
    CdataSection,
}
`))
	var buf strings.Builder
	t.Execute(&buf, cat)
	return buf.String()
}

func genEvents(cat *pb.Catalog) string {
	t := template.Must(template.New("evt").Funcs(template.FuncMap{"toRustIdent": toRustIdent}).Parse(`// Auto-generated from spec catalog.
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    None = 0,
    CapturingPhase = 1,
    AtTarget = 2,
    BubblingPhase = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
	{{- range .DomEvents }}
    {{ .Name | toRustIdent }},
	{{- end }}
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
		{{- range .DomEvents }}
            EventType::{{ .Name | toRustIdent }} => "{{ .Name }}",
		{{- end }}
        }
    }
}

impl FromStr for EventType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
		{{- range .DomEvents }}
            "{{ .Name }}" => Ok(EventType::{{ .Name | toRustIdent }}),
		{{- end }}
            _ => Err(()),
        }
    }
}
`))
	var buf strings.Builder
	t.Execute(&buf, cat)
	return buf.String()
}

func toRustIdent(s string) string {
	// Convert "kebab-case" or "snake_case" to "PascalCase" for Rust identifiers
	result := ""
	capNext := true
	for _, c := range s {
		if c == '-' || c == '_' || c == ' ' {
			capNext = true
		} else {
			if capNext {
				result += string(c - 'a' + 'A')
				capNext = false
			} else {
				result += string(c)
			}
		}
	}
	// Escape Rust keywords
	if result == "Type" {
		return "Type_"
	}
	return result
}
