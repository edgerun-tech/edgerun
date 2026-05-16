package spec

import (
	"os"
	"strings"

	"github.com/spf13/cobra"

	pb "edgerunrefcore/proto/go/spec"
)

// ---- HTML Extractor ----

func runHTML(cmd *cobra.Command, args []string) error {
	data, err := os.ReadFile(args[0])
	if err != nil {
		return err
	}
	md := string(data)
	cat := &pb.Catalog{SpecName: "WHATWG HTML Living Standard", SpecDate: "2026-04-10"}

	// Extract element definitions from IDL blocks
	// Pattern: interface HTMLElementName : ParentName { ... };
	matches := regexFindAllSubmatch(`interface\s+(HTML\w+)\s*:\s*(\w+)\s*\{(.*?)\};`, md)
	for _, m := range matches {
		elem := &pb.HtmlElement{
			TagName:      strings.ToLower(strings.TrimPrefix(m[1], "HTML")),
			DomInterface: m[1],
		}
		cat.HtmlElements = append(cat.HtmlElements, elem)
	}

	// Extract element-specific attributes from attribute lines
	attrMatches := regexFindAllSubmatch(`attribute\s+(\w+)\s+(\w+)`, md)
	for _, am := range attrMatches {
		for _, elem := range cat.HtmlElements {
			elem.ElementAttrs = append(elem.ElementAttrs, &pb.HtmlAttrDef{
				Name: am[2],
			})
		}
	}

	cat.TotalElements = int32(len(cat.HtmlElements))
	return writeProto(cat)
}

// ---- ECMAScript Extractor ----

func runECMAScript(cmd *cobra.Command, args []string) error {
	data, err := os.ReadFile(args[0])
	if err != nil {
		return err
	}
	md := string(data)
	cat := &pb.Catalog{SpecName: "ECMA-262 (ECMAScript)"}

	// Extract built-in objects: %ObjectName% or plain ObjectName { ... }
	builtinMatches := regexFindAllSubmatch(`(?:%\w+%|\b(\w+))\s*(?:=|\{)((?:.|\n)*?)\}(?:\s*(?:;|$))`, md)
	for _, m := range builtinMatches {
		obj := &pb.JsBuiltInObjectDef{Name: m[1]}
		body := m[2]

		// Extract static methods: ObjectName.methodName(
		staticMatches := regexFindAllSubmatch(m[1]+`\.(\w+)\s*\(([^)]*)\)`, body)
		for _, sm := range staticMatches {
			obj.StaticMethods = append(obj.StaticMethods, &pb.JsMethodDef{
				Name:       sm[1],
				Parameters: strings.Split(strings.ReplaceAll(sm[2], " ", ""), ","),
			})
		}

		// Extract prototype methods: ObjectName.prototype.methodName(
		protoMatches := regexFindAllSubmatch(m[1]+`\.prototype\.(\w+)\s*\(([^)]*)\)`, body)
		for _, pm := range protoMatches {
			obj.PrototypeMethods = append(obj.PrototypeMethods, &pb.JsMethodDef{
				Name:       pm[1],
				Parameters: strings.Split(strings.ReplaceAll(pm[2], " ", ""), ","),
			})
		}

		cat.JsBuiltins = append(cat.JsBuiltins, obj)
	}

	// Extract abstract operations: AbstractOpName ( params )
	aoMatches := regexFindAllSubmatch(`(\w+)\s*\(\s*([^)]*)\s*\)\s*\n`, md)
	for _, am := range aoMatches {
		name := am[1]
		if isBuiltinName(name) {
			continue
		}
		cat.JsAbstractOps = append(cat.JsAbstractOps, &pb.JsAbstractOpDef{
			Name:       name,
			Parameters: splitParams(am[2]),
		})
	}

	// Well-known symbols
	for _, s := range wellKnownSymbols {
		cat.JsWellKnownSymbols = append(cat.JsWellKnownSymbols, s)
	}
	// Well-known intrinsics
	for _, i := range wellKnownIntrinsics {
		cat.JsWellKnownIntrinsics = append(cat.JsWellKnownIntrinsics, i)
	}

	return writeProto(cat)
}

var wellKnownSymbols = []string{
	"asyncIterator", "hasInstance", "isConcatSpreadable", "iterator",
	"match", "matchAll", "replace", "search", "species", "split",
	"toPrimitive", "toStringTag", "unscopables",
}

var wellKnownIntrinsics = []string{
	"%AggregateError%", "%Array%", "%ArrayBuffer%", "%AsyncFunction%",
	"%Atomics%", "%BigInt%", "%BigInt64Array%", "%BigUint64Array%",
	"%Boolean%", "%DataView%", "%Date%", "%Error%", "%EvalError%",
	"%FinalizationRegistry%", "%Float32Array%", "%Float64Array%",
	"%Function%", "%Generator%", "%GeneratorFunction%", "%Int8Array%",
	"%Int16Array%", "%Int32Array%", "%JSON%", "%Map%", "%Number%",
	"%Object%", "%Promise%", "%Proxy%", "%RangeError%", "%ReferenceError%",
	"%Reflect%", "%RegExp%", "%Set%", "%SharedArrayBuffer%", "%String%",
	"%Symbol%", "%SyntaxError%", "%TypeError%", "%Uint8Array%",
	"%Uint8ClampedArray%", "%Uint16Array%", "%Uint32Array%", "%URIError%",
	"%WeakMap%", "%WeakRef%", "%WeakSet%",
}

func isBuiltinName(name string) bool {
	builtins := []string{"Object", "Array", "String", "Number", "Boolean", "Function", "Symbol", "Error", "Math", "Date", "RegExp", "JSON", "Promise", "Map", "Set", "WeakMap", "Set", "ArrayBuffer", "DataView", "Int8Array", "Uint8Array", "Int16Array", "Uint16Array", "Int32Array", "Uint32Array", "Float32Array", "Float64Array", "BigInt", "BigInt64Array", "BigUint64Array"}
	for _, b := range builtins {
		if b == name {
			return true
		}
	}
	return false
}

func splitParams(s string) []string {
	s = strings.TrimSpace(s)
	if s == "" {
		return nil
	}
	var params []string
	for _, p := range strings.Split(s, ",") {
		p = strings.TrimSpace(p)
		if p != "" {
			params = append(params, p)
		}
	}
	return params
}
