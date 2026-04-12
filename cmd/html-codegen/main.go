// html-codegen reads proto IR text files and generates Rust parser source.
//
// Usage: go run ./cmd/html-codegen [data-dir] [output-dir]
//
// Reads:
//   data-dir/tokenizer.textproto
//   data-dir/tree_builder.textproto
//   data-dir/entities.textproto
//   data-dir/element_metadata.json
//
// Writes:
//   output-dir/crates/edgerun-render/src/html/tokenizer.rs
//   output-dir/crates/edgerun-render/src/html/tree_builder.rs
//   output-dir/crates/edgerun-render/src/html/entity_decoder/mod.rs
//   output-dir/crates/edgerun-render/src/html/entity_decoder/table.inc
//   output-dir/crates/edgerun-render/src/html/html_parser.rs
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	codegen "edgerunrefcore/cmd/html-codegen/codegen"
	"edgerunrefcore/gen/go/edgerun/v0/html"
	"google.golang.org/protobuf/encoding/prototext"
)

func main() {
	dataDir := "data"
	outDir := "."
	if len(os.Args) > 1 {
		dataDir = os.Args[1]
	}
	if len(os.Args) > 2 {
		outDir = os.Args[2]
	}

	// Read IR files
	tokenizerData := readFile(filepath.Join(dataDir, "tokenizer.textproto"))
	treeBuilderData := readFile(filepath.Join(dataDir, "tree_builder.textproto"))
	entitiesData := readFile(filepath.Join(dataDir, "entities.textproto"))
	metadataData := readFile(filepath.Join(dataDir, "element_metadata.json"))

	// Unmarshal proto messages
	var machine html.TokenizerStateMachine
	if err := prototext.Unmarshal(tokenizerData, &machine); err != nil {
		fatalf("failed to parse tokenizer.textproto: %v", err)
	}

	var ruleSet html.TreeBuilderRuleSet
	if err := prototext.Unmarshal(treeBuilderData, &ruleSet); err != nil {
		fatalf("failed to parse tree_builder.textproto: %v", err)
	}

	var catalog html.EntityCatalog
	if err := prototext.Unmarshal(entitiesData, &catalog); err != nil {
		fatalf("failed to parse entities.textproto: %v", err)
	}

	var metadata map[string][]string
	if err := json.Unmarshal(metadataData, &metadata); err != nil {
		fatalf("failed to parse element_metadata.json: %v", err)
	}

	// Generate Rust source files
	srcDir := filepath.Join(outDir, "crates", "edgerun-html-render", "src")
	os.MkdirAll(srcDir, 0755)

	tokenizerRs := codegen.GenerateTokenizerRust(&machine, metadata)
	writeFile(filepath.Join(srcDir, "tokenizer.rs"), tokenizerRs)

	treeBuilderRs := codegen.GenerateTreeBuilderRust(&ruleSet, metadata["void_elements"])
	writeFile(filepath.Join(srcDir, "tree_builder.rs"), treeBuilderRs)

	// Entity decoder is generated as a multi-file module (mod.rs + table.inc)
	entityFiles := codegen.GenerateEntityDecoderRust(&catalog)
	for relPath, content := range entityFiles {
		fullPath := filepath.Join(srcDir, relPath)
		os.MkdirAll(filepath.Dir(fullPath), 0755)
		writeFile(fullPath, content)
	}

	// Remove old single-file entity_decoder.rs if it exists
	os.Remove(filepath.Join(srcDir, "entity_decoder.rs"))

	htmlParserRs := codegen.GenerateHtmlParserRs()
	writeFile(filepath.Join(srcDir, "html_parser.rs"), htmlParserRs)

	totalFiles := 3 + len(entityFiles)
	fmt.Printf("Generated %d Rust files to %s\n", totalFiles, srcDir)
}

func readFile(path string) []byte {
	data, err := os.ReadFile(path)
	if err != nil {
		fatalf("failed to read %s: %v", path, err)
	}
	return data
}

func writeFile(path, content string) {
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		fatalf("failed to write %s: %v", path, err)
	}
	rel := path
	if wd, err := os.Getwd(); err == nil {
		if r, err := filepath.Rel(wd, path); err == nil {
			rel = r
		}
	}
	fmt.Printf("  Written: %s\n", rel)
}

func fatalf(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, "error: "+format+"\n", args...)
	os.Exit(1)
}
