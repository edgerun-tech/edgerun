// Generate protobuf definitions for ECMAScript built-in objects from catalog JSON.
//
// Usage: go run ./cmd/generate-ecmascript-proto scripts/ecmascript_catalog.json proto/edgerun/v0/ecmascript/
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

type JSCatalog struct {
	BuiltInObjects   []JSObj  `json:"built_in_objects"`
	AbstractOps      []JSOp   `json:"abstract_operations"`
	WellKnownSymbols []string `json:"well_known_symbols"`
	WellKnownIntrinsics []string `json:"well_known_intrinsics"`
}

type JSObj struct {
	Name             string     `json:"name"`
	PrototypeMethods []JSMethod `json:"prototype_methods"`
	StaticProperties []JSProp   `json:"static_properties"`
}

type JSMethod struct {
	Name       string   `json:"name"`
	Parameters []string `json:"parameters"`
	Section    string   `json:"section"`
}

type JSProp struct {
	Name string `json:"name"`
	Kind string `json:"kind"`
}

type JSOp struct {
	Name       string   `json:"name"`
	Parameters []string `json:"parameters"`
	Section    string   `json:"section"`
}

func toUpperSnake(s string) string {
	s = regexp.MustCompile(`([a-z])([A-Z])`).ReplaceAllString(s, `${1}_${2}`)
	s = strings.ReplaceAll(s, "-", "_")
	return strings.ToUpper(s)
}

func genObjectsProto(c *JSCatalog) string {
	objs := make([]JSObj, 0, len(c.BuiltInObjects))
	for _, o := range c.BuiltInObjects {
		if len(o.PrototypeMethods) > 0 || len(o.StaticProperties) > 0 {
			objs = append(objs, o)
		}
	}

	var L []string
	L = append(L, `syntax = "proto3";`,
		``,
		`package edgerun.v0.ecmascript.objects;`,
		``,
		`/// ECMAScript Built-in Object Definitions -- generated from ECMA-262.`,
		`/// DO NOT EDIT. Regenerate with: go run ./cmd/generate-ecmascript-proto`,
		``,
		`/// All ECMAScript built-in objects.`,
		fmt.Sprintf(`/// Total: %d objects.`, len(objs)),
		`enum JsBuiltInObject {`,
		`  JS_BUILT_IN_OBJECT_UNSPECIFIED = 0;`)

	for _, o := range objs {
		enum := toUpperSnake(o.Name)
		pm := len(o.PrototypeMethods)
		sp := len(o.StaticProperties)
		L = append(L, fmt.Sprintf(`  %s = %d; // %s (%d methods, %d static props)`, enum, len(L), o.Name, pm, sp))
	}

	L = append(L, `}`, ``,
		`/// An ECMAScript built-in method.`,
		`message JsMethod {`,
		`  string name = 1;`,
		`  repeated string parameters = 2;`,
		`  string section = 3;`,
		`  bool is_prototype_method = 4;`,
		`  bool is_static = 5;`,
		`}`, ``,
		`/// An ECMAScript built-in property.`,
		`message JsProperty {`,
		`  string name = 1;`,
		`  string kind = 2;`,
		`  string section = 3;`,
		`}`, ``,
		`/// Complete definition of an ECMAScript built-in object.`,
		`message JsBuiltInObjectDef {`,
		`  JsBuiltInObject object = 1;`,
		`  string name = 2;`,
		`  repeated JsMethod prototype_methods = 3;`,
		`  repeated JsMethod static_methods = 4;`,
		`  repeated JsProperty prototype_properties = 5;`,
		`  repeated JsProperty static_properties = 6;`,
		`  string section = 7;`,
		`}`, ``)

	return strings.Join(L, "\n")
}

func genAbstractOpsProto(c *JSCatalog) string {
	var L []string
	L = append(L, `syntax = "proto3";`,
		``,
		`package edgerun.v0.ecmascript.abstract_ops;`,
		``,
		`/// ECMAScript Abstract Operation Definitions -- generated from ECMA-262.`,
		`/// DO NOT EDIT. Regenerate with: go run ./cmd/generate-ecmascript-proto`,
		``,
		`/// All ECMAScript abstract operations.`,
		fmt.Sprintf(`/// Total: %d operations.`, len(c.AbstractOps)),
		`enum JsAbstractOperation {`,
		`  JS_ABSTRACT_OPERATION_UNSPECIFIED = 0;`)

	for _, op := range c.AbstractOps {
		enum := toUpperSnake(op.Name)
		params := strings.Join(op.Parameters, ", ")
		L = append(L, fmt.Sprintf(`  %s = %d; // %s(%s)`, enum, len(L), op.Name, params))
	}

	L = append(L, `}`, ``,
		`/// Definition of an ECMAScript abstract operation.`,
		`message JsAbstractOpDef {`,
		`  JsAbstractOperation operation = 1;`,
		`  string name = 2;`,
		`  repeated string parameters = 3;`,
		`  string section = 4;`,
		`}`, ``)

	return strings.Join(L, "\n")
}

func genGlobalsProto(c *JSCatalog) string {
	symbols := c.WellKnownSymbols
	intrinsics := c.WellKnownIntrinsics

	// Filter intrinsics
	symUpper := make(map[string]bool)
	for _, s := range symbols {
		symUpper[strings.ToUpper(s)] = true
	}
	var filteredIntrinsics []string
	seen := make(map[string]bool)
	for _, intrinsic := range intrinsics {
		key := strings.ToUpper(strings.ReplaceAll(intrinsic, ".", "_"))
		if seen[key] {
			continue
		}
		seen[key] = true
		parts := strings.Split(intrinsic, ".")
		if len(parts) >= 2 && parts[0] == "Symbol" && symUpper[strings.ToUpper(parts[len(parts)-1])] {
			continue
		}
		filteredIntrinsics = append(filteredIntrinsics, intrinsic)
	}

	var L []string
	L = append(L, `syntax = "proto3";`,
		``,
		`package edgerun.v0.ecmascript.globals;`,
		``,
		`/// ECMAScript Global Symbols and Intrinsics -- generated from ECMA-262.`,
		`/// DO NOT EDIT. Regenerate with: go run ./cmd/generate-ecmascript-proto`,
		``,
		`/// ECMAScript well-known symbols (@@iterator, @@toStringTag, etc.).`,
		fmt.Sprintf(`/// Total: %d symbols.`, len(symbols)),
		`enum JsWellKnownSymbol {`,
		`  JS_WELL_KNOWN_SYMBOL_UNSPECIFIED = 0;`)

	for _, sym := range symbols {
		enum := "SYMBOL_" + toUpperSnake(sym)
		L = append(L, fmt.Sprintf(`  %s = %d; // @@%s`, enum, len(L), sym))
	}

	L = append(L, `}`, ``,
		`/// ECMAScript well-known intrinsic objects (%Array%, %Object%, etc.).`,
		fmt.Sprintf(`/// Total: %d intrinsics.`, len(filteredIntrinsics)),
		`enum JsIntrinsic {`,
		`  JS_INTRINSIC_UNSPECIFIED = 0;`)

	for _, intrinsic := range filteredIntrinsics {
		if len(intrinsic) > 80 {
			continue
		}
		enum := "INTRINSIC_" + toUpperSnake(strings.ReplaceAll(intrinsic, ".", "_"))
		if len(enum) > 0 && enum[0] >= '0' && enum[0] <= '9' {
			enum = "I" + enum
		}
		L = append(L, fmt.Sprintf(`  %s = %d; // %%%s%%`, enum, len(L), intrinsic))
	}

	L = append(L, `}`, ``)

	return strings.Join(L, "\n")
}

func main() {
	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "Usage: generate-ecmascript-proto <catalog.json> <output_dir>")
		os.Exit(1)
	}

	data, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	var catalog JSCatalog
	if err := json.Unmarshal(data, &catalog); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	outDir := os.Args[2]
	os.MkdirAll(outDir, 0755)

	files := map[string]string{
		"ecmascript_objects.proto":   genObjectsProto(&catalog),
		"ecmascript_abstract_ops.proto": genAbstractOpsProto(&catalog),
		"ecmascript_globals.proto":   genGlobalsProto(&catalog),
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
