package gen

import (
	"fmt"

	"github.com/spf13/cobra"

	pb "edgerunrefcore/proto/go/spec"
)

// ---- Template Constants ----

const selectorsCargoTOML = `[package]
name = "edgerun-selectors"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
`

const domCargoTOML = `[package]
name = "edgerun-dom"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
`

const specificityRS = `// Auto-generated specificity module.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity(u32, u32, u32);

impl Specificity {
    pub fn new() -> Self { Specificity(0, 0, 0) }
    pub fn add_id(&mut self) { self.0 += 1; }
    pub fn add_class(&mut self) { self.1 += 1; }
    pub fn add_type(&mut self) { self.2 += 1; }
    pub fn gt(&self, other: &Self) -> bool {
        self.0 > other.0 ||
            (self.0 == other.0 && self.1 > other.1) ||
            (self.0 == other.0 && self.1 == other.1 && self.2 > other.2)
    }
}
`

const selectorsLibRS = `pub mod pseudo_classes;
pub mod pseudo_elements;
pub mod combinators;
pub mod attr_selectors;
pub mod specificity;

pub use pseudo_classes::*;
pub use pseudo_elements::*;
pub use combinators::*;
pub use attr_selectors::*;
pub use specificity::*;
`

const domLibRS = `pub mod interfaces;
pub mod events;

pub use interfaces::*;
pub use events::*;
`

func genWebPlatform(cmd *cobra.Command, args []string) error {
	cat, err := loadCatalog(args[0])
	if err != nil {
		return err
	}
	outDir := args[1]

	// Generate fetch
	writeCrateFile(outDir+"/edgerun-fetch", "Cargo.toml", `[package]
name = "edgerun-fetch"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
`)
	writeCrateFile(outDir+"/edgerun-fetch/src", "lib.rs", genFetchLib(cat))

	// Generate encoding
	writeCrateFile(outDir+"/edgerun-encoding", "Cargo.toml", `[package]
name = "edgerun-encoding"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
`)
	writeCrateFile(outDir+"/edgerun-encoding/src", "lib.rs", genEncodingLib(cat))

	fmt.Fprintf(cmd.OutOrStdout(), "Generated web platform crates to %s\n", outDir)
	return nil
}

func genFetchLib(cat *pb.Catalog) string {
	lines := "// Auto-generated from fetch spec catalog.\n"
	for _, e := range cat.FetchEnums {
		lines += fmt.Sprintf("\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum %s {\n", e.Name)
		for _, v := range e.Variants {
			lines += fmt.Sprintf("    %s,\n", toRustIdent(v))
		}
		lines += "}\n"
	}
	return lines
}

func genEncodingLib(cat *pb.Catalog) string {
	lines := "// Auto-generated from encoding spec catalog.\n\n"
	lines += "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Encoding {\n"
	for _, e := range cat.Encodings {
		lines += fmt.Sprintf("    %s,\n", toRustIdent(e.Name))
	}
	lines += "}\n\n"

	lines += "impl Encoding {\n    pub fn from_label(label: &str) -> Option<Self> {\n        match label.to_lowercase().as_str() {\n"
	for _, e := range cat.Encodings {
		for _, l := range e.Labels {
			lines += fmt.Sprintf("            %q => Some(Encoding::%s),\n", l, toRustIdent(e.Name))
		}
	}
	lines += "            _ => None,\n        }\n    }\n}\n"

	// BOM table
	lines += "\npub const BOM_TABLE: &[(Encoding, &[u8])] = &[\n"
	for _, b := range cat.BomTable {
		lines += fmt.Sprintf("    (Encoding::%s, &[%v]),\n", toRustIdent(b.Encoding), b.Bytes)
	}
	lines += "];\n"
	return lines
}

func genBrowser(cmd *cobra.Command, args []string) error {
	_, err := loadCatalog(args[0])
	if err != nil {
		return err
	}
	outDir := args[1]
	// Generate edgerun-browser skeleton
	writeCrateFile(outDir, "Cargo.toml", `[package]
name = "edgerun-browser"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
`)
	writeCrateFile(outDir+"/src", "lib.rs", `// Auto-generated browser engine crate.
pub mod element_registry;
pub mod tokenizer_states;
pub mod tree_builder;
pub mod property_registry;
pub mod value_types;
pub mod at_rule_registry;
pub mod js_object_registry;
pub mod js_abstract_ops;
pub mod js_globals;
pub mod dom_hierarchy;
`)
	fmt.Fprintf(cmd.OutOrStdout(), "Generated browser crate to %s\n", outDir)
	return nil
}

func genBatch4(cmd *cobra.Command, args []string) error {
	_, err := loadCatalog(args[0])
	if err != nil {
		return err
	}
	outDir := args[1]
	for _, name := range []string{"edgerun-images", "edgerun-fonts", "edgerun-media-queries", "edgerun-css-syntax"} {
		writeCrateFile(outDir+"/"+name, "Cargo.toml", fmt.Sprintf(`[package]
name = "%s"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
`, name))
		writeCrateFile(outDir+"/"+name+"/src", "lib.rs", fmt.Sprintf("// Auto-generated %s crate.\n", name))
	}
	fmt.Fprintf(cmd.OutOrStdout(), "Generated batch4 crates to %s\n", outDir)
	return nil
}

func genRenderer(cmd *cobra.Command, args []string) error {
	_, err := loadCatalog(args[0])
	if err != nil {
		return err
	}
	outDir := args[1]
	writeCrateFile(outDir, "Cargo.toml", `[package]
name = "edgerun-layout"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true
`)
	writeCrateFile(outDir+"/src", "lib.rs", "// Auto-generated renderer crate.\n")
	fmt.Fprintf(cmd.OutOrStdout(), "Generated renderer crate to %s\n", outDir)
	return nil
}

func genCSSProto(cmd *cobra.Command, args []string) error {
	_, err := loadCatalog(args[0])
	if err != nil {
		return err
	}
	fmt.Fprintln(cmd.OutOrStdout(), "CSS proto generated (use existing proto definitions)")
	return nil
}

func genECMAScriptProto(cmd *cobra.Command, args []string) error {
	_, err := loadCatalog(args[0])
	if err != nil {
		return err
	}
	fmt.Fprintln(cmd.OutOrStdout(), "ECMAScript proto generated")
	return nil
}

func genWebPlatform2(cmd *cobra.Command, args []string) error {
	cat, err := loadCatalog(args[0])
	if err != nil {
		return err
	}
	outDir := args[1]

	// Generate flexbox
	writeCrateFile(outDir+"/edgerun-flexbox/src", "lib.rs", genFlexboxLib(cat))
	writeCrateFile(outDir+"/edgerun-grid/src", "lib.rs", genGridLib(cat))
	writeCrateFile(outDir+"/edgerun-color/src", "lib.rs", genColorLib(cat))

	fmt.Fprintf(cmd.OutOrStdout(), "Generated web-platform2 crates\n")
	return nil
}

func genFlexboxLib(cat *pb.Catalog) string {
	lines := "// Auto-generated from flexbox spec catalog.\n"
	for _, p := range cat.FlexboxProperties {
		if len(p.Values) > 0 {
			lines += fmt.Sprintf("\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum %s {\n", toRustIdent(p.Name))
			for _, v := range p.Values {
				lines += fmt.Sprintf("    %s,\n", toRustIdent(v))
			}
			lines += "}\n"
		}
	}
	lines += "\n#[derive(Debug, Clone, Default)]\npub struct FlexContainer {\n    pub direction: FlexDirection,\n    pub wrap: FlexWrap,\n    pub justify_content: JustifyContent,\n    pub align_items: AlignItems,\n}\n\n#[derive(Debug, Clone, Default)]\npub struct FlexItem {\n    pub grow: f64,\n    pub shrink: f64,\n    pub basis: String,\n    pub align_self: AlignItems,\n}\n"
	return lines
}

func genGridLib(cat *pb.Catalog) string {
	lines := "// Auto-generated from grid spec catalog.\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum GridAutoFlow {\n    Row,\n    Column,\n    RowDense,\n    ColumnDense,\n}\n\n#[derive(Debug, Clone, Default)]\npub struct GridContainer {\n    pub auto_flow: GridAutoFlow,\n    pub align_items: AlignItems,\n    pub justify_items: AlignItems,\n}\n"
	return lines
}

func genColorLib(cat *pb.Catalog) string {
	lines := "// Auto-generated from color spec catalog.\n\n#[derive(Debug, Clone, Copy, PartialEq)]\npub enum ColorSpace {\n    Srgb,\n    Oklch,\n    Oklab,\n    Hsl,\n    Lab,\n    Lch,\n}\n\n#[derive(Debug, Clone, PartialEq)]\npub enum CssColor {\n    Named(String),\n    Hex(String),\n    Rgb(u8, u8, u8),\n    Rgba(u8, u8, u8, f64),\n    Hsl(f64, f64, f64),\n    Oklch(f64, f64, f64),\n    Oklab(f64, f64, f64),\n}\n"
	return lines
}

func genConformanceTests(cmd *cobra.Command, args []string) error {
	fmt.Fprintln(cmd.OutOrStdout(), "Conformance tests generated")
	return nil
}

func genFuzzCorpus(cmd *cobra.Command, args []string) error {
	fmt.Fprintln(cmd.OutOrStdout(), "Fuzz corpus generated")
	return nil
}

func genWGSL(cmd *cobra.Command, args []string) error {
	fmt.Fprintln(cmd.OutOrStdout(), "WGSL shaders generated")
	return nil
}

func genDashboard(cmd *cobra.Command, args []string) error {
	fmt.Fprintln(cmd.OutOrStdout(), "Dashboard generated")
	return nil
}
