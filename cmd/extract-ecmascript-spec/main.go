// Extract ECMAScript built-in objects, abstract operations, symbols, intrinsics from spec markdown.
//
// Usage: go run ./cmd/extract-ecmascript-spec docs/ecmascript_spec.md > scripts/ecmascript_catalog.json
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"
)

type JSMethod struct {
	Name       string   `json:"name"`
	Parameters []string `json:"parameters"`
	Section    string   `json:"section"`
}

type JSProperty struct {
	Name    string `json:"name"`
	Kind    string `json:"kind"`
	Section string `json:"section"`
}

type JSBuiltInObject struct {
	Name                   string       `json:"name"`
	ConstructorSignature   *string      `json:"constructor_signature,omitempty"`
	StaticMethods          []JSMethod   `json:"static_methods"`
	PrototypeMethods       []JSMethod   `json:"prototype_methods"`
	StaticProperties       []JSProperty `json:"static_properties"`
	PrototypeProperties    []JSProperty `json:"prototype_properties"`
}

type JSAbstractOp struct {
	Name       string   `json:"name"`
	Parameters []string `json:"parameters"`
	Section    string   `json:"section"`
}

type JSGrammarProduction struct {
	NonTerminal string `json:"non_terminal"`
	Production  string `json:"production"`
}

type JSCatalog struct {
	Spec                  string              `json:"spec"`
	SpecDate              string              `json:"spec_date"`
	TotalBuiltInObjects   int                 `json:"total_built_in_objects"`
	TotalAbstractOps      int                 `json:"total_abstract_operations"`
	TotalWellKnownSymbols int                 `json:"total_well_known_symbols"`
	TotalWellKnownIntrinsics int               `json:"total_well_known_intrinsics"`
	BuiltInObjects        []JSBuiltInObject   `json:"built_in_objects"`
	AbstractOps           []JSAbstractOp      `json:"abstract_operations"`
	WellKnownSymbols      []string            `json:"well_known_symbols"`
	WellKnownIntrinsics   []string            `json:"well_known_intrinsics"`
	GrammarProductions    []JSGrammarProduction `json:"grammar_productions"`
}

func cleanParams(s string) []string {
	s = strings.ReplaceAll(s, "`", "")
	s = strings.TrimSpace(s)
	if s == "" {
		return nil
	}
	s = regexp.MustCompile(`\[\s*,?\s*`).ReplaceAllString(s, "[")
	var params []string
	for _, p := range strings.Split(s, ",") {
		p = strings.TrimSpace(p)
		if p != "" {
			params = append(params, p)
		}
	}
	return params
}

func bodyAround(text string, start, maxLen int) string {
	end := start + maxLen
	if end > len(text) {
		end = len(text)
	}
	return text[start:end]
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: extract-ecmascript-spec <path-to-ecmascript-spec.md>")
		os.Exit(1)
	}
	data, err := os.ReadFile(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	md := string(data)

	objects := make(map[string]*JSBuiltInObject)

	// Prototype methods: # <span class="secnum">X.Y.Z</span> Obj.prototype.method ( params )
	methodRe := regexp.MustCompile(`# <span class="secnum">([^<]+)</span>\s+([A-Z][\w]+)\.prototype\.(\w+)\s*\(\s*([^)]*)\)`)
	for _, m := range methodRe.FindAllStringSubmatch(md, -1) {
		secNum, objName, methodName, params := m[1], m[2], m[3], cleanParams(m[4])
		if _, ok := objects[objName]; !ok {
			objects[objName] = &JSBuiltInObject{Name: objName}
		}
		objects[objName].PrototypeMethods = append(objects[objName].PrototypeMethods, JSMethod{
			Name: methodName, Parameters: params, Section: secNum,
		})
	}

	// Constructors: # <span class="secnum">X.Y.Z</span> Obj ( ...params )
	ctorRe := regexp.MustCompile(`# <span class="secnum">([^<]+)</span>\s+([A-Z][\w]+)\s*\(\s*([^)]*)\)`)
	for _, m := range ctorRe.FindAllStringSubmatch(md, -1) {
		secNum, objName, params := m[1], m[2], cleanParams(m[3])
		parts := strings.Split(secNum, ".")
		if len(parts) >= 2 {
			// Skip if already captured as a prototype method call
			if _, ok := objects[objName]; !ok {
				objects[objName] = &JSBuiltInObject{Name: objName}
			}
			if objects[objName].ConstructorSignature == nil {
				sig := fmt.Sprintf("%s(%s)", objName, strings.Join(params, ", "))
				objects[objName].ConstructorSignature = &sig
			}
		}
	}

	// Accessors: get/set Obj[prop]
	accessorRe := regexp.MustCompile(`# <span class="secnum">([^<]+)</span>\s+(get|set)\s+([A-Z][\w]+)\[([^\]]+)\]`)
	for _, m := range accessorRe.FindAllStringSubmatch(md, -1) {
		secNum, kind, objName, propName := m[1], m[2], m[3], strings.TrimSpace(strings.Trim(m[4], "%"))
		if _, ok := objects[objName]; !ok {
			objects[objName] = &JSBuiltInObject{Name: objName}
		}
		objects[objName].StaticProperties = append(objects[objName].StaticProperties, JSProperty{
			Name: propName, Kind: kind, Section: secNum,
		})
	}

	// Prototype properties: # <span class="secnum">X.Y.Z</span> Obj.prototype.prop
	propRe := regexp.MustCompile(`# <span class="secnum">([^<]+)</span>\s+([A-Z][\w]+)\.prototype\.(\w+)`)
	for _, m := range propRe.FindAllStringSubmatch(md, -1) {
		secNum, objName, propName := m[1], m[2], m[3]
		if obj, ok := objects[objName]; ok {
			existing := make(map[string]bool)
			for _, pm := range obj.PrototypeMethods {
				existing[pm.Name] = true
			}
			for _, pp := range obj.PrototypeProperties {
				existing[pp.Name] = true
			}
			if !existing[propName] {
				obj.PrototypeProperties = append(obj.PrototypeProperties, JSProperty{
					Name: propName, Section: secNum,
				})
			}
		}
	}

	// Static properties: # <span class="secnum">X.Y.Z</span> Obj.prop
	staticPropRe := regexp.MustCompile(`# <span class="secnum">([^<]+)</span>\s+([A-Z][\w]+)\.(\w+)\s*$`)
	for _, m := range staticPropRe.FindAllStringSubmatch(md, -1) {
		secNum, objName, propName := m[1], m[2], m[3]
		if obj, ok := objects[objName]; ok {
			existing := make(map[string]bool)
			for _, sp := range obj.StaticProperties {
				existing[sp.Name] = true
			}
			for _, pm := range obj.PrototypeMethods {
				existing[pm.Name] = true
			}
			if !existing[propName] {
				obj.StaticProperties = append(obj.StaticProperties, JSProperty{
					Name: propName, Section: secNum,
				})
			}
		}
	}

	var builtInObjects []JSBuiltInObject
	for _, obj := range objects {
		builtInObjects = append(builtInObjects, *obj)
	}

	// Abstract operations
	absRe := regexp.MustCompile(`# <span class="secnum">([^<]+)</span>\s+(\w+)\s*\(\s*([^)]*)\)`)
	seenOps := make(map[string]bool)
	var absOps []JSAbstractOp
	for _, m := range absRe.FindAllStringSubmatch(md, -1) {
		secNum := m[1]
		name := m[2]
		params := cleanParams(m[3])
		parts := strings.Split(secNum, ".")
		if len(parts) >= 1 && (parts[0] == "6" || parts[0] == "7") {
			skipWords := map[string]bool{"if": true, "for": true, "while": true, "return": true, "let": true, "const": true, "var": true, "function": true, "class": true}
			if skipWords[name] || seenOps[name] {
				continue
			}
			seenOps[name] = true
			absOps = append(absOps, JSAbstractOp{Name: name, Parameters: params, Section: secNum})
		}
	}

	// Well-known symbols
	symRe := regexp.MustCompile(`@@([\w]+)`)
	syms := make(map[string]bool)
	for _, m := range symRe.FindAllStringSubmatch(md, -1) {
		syms[m[1]] = true
	}
	var symbols []string
	for s := range syms {
		symbols = append(symbols, s)
	}
	sort.Strings(symbols)

	// Intrinsics
	intRe := regexp.MustCompile(`%([\w.]+)%`)
	intrinsics := make(map[string]bool)
	for _, m := range intRe.FindAllStringSubmatch(md, -1) {
		intrinsics[m[1]] = true
	}
	var intr []string
	for s := range intrinsics {
		intr = append(intr, s)
	}
	sort.Strings(intr)

	// Grammar productions
	gramRe := regexp.MustCompile("`([A-Z][A-Za-z]+)`\\s*:\\s*\\n\\s*`([A-Z][A-Za-z]+)`")
	var grammar []JSGrammarProduction
	for _, m := range gramRe.FindAllStringSubmatch(md, -1) {
		grammar = append(grammar, JSGrammarProduction{NonTerminal: m[1], Production: m[2]})
	}

	catalog := JSCatalog{
		Spec:                    "ECMA-262 (ECMAScript)",
		SpecDate:                "2026-04-10",
		TotalBuiltInObjects:     len(builtInObjects),
		TotalAbstractOps:        len(absOps),
		TotalWellKnownSymbols:   len(symbols),
		TotalWellKnownIntrinsics: len(intr),
		BuiltInObjects:         builtInObjects,
		AbstractOps:            absOps,
		WellKnownSymbols:       symbols,
		WellKnownIntrinsics:    intr,
		GrammarProductions:     grammar,
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
