// Extract Web IDL spec: type system, extended attributes, callback rules.
package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type ExtAttr struct {
	Name        string `json:"name"`
	Description string `json:"description"`
}

type CallbackRule struct {
	Name        string `json:"name"`
	Description string `json:"description"`
}

type NullBehavior struct {
	Name        string `json:"name"`
	Description string `json:"description"`
}

type WebIDLCatalog struct {
	Spec             string       `json:"spec"`
	SpecDate         string       `json:"spec_date"`
	PrimitiveTypes   []string     `json:"primitive_types"`
	StringTypes      []string     `json:"string_types"`
	ObjectTypes      []string     `json:"object_types"`
	SpecialTypes     []string     `json:"special_types"`
	ExtendedAttrs    []ExtAttr    `json:"extended_attributes"`
	CallbackRules    []CallbackRule `json:"callback_rules"`
	NullBehavior     []NullBehavior `json:"null_behavior"`
}

func main() {
	if len(os.Args) >= 2 {
		if _, err := os.ReadFile(os.Args[1]); err != nil {
			fmt.Fprintf(os.Stderr, "error: %v\n", err)
			os.Exit(1)
		}
	}

	catalog := WebIDLCatalog{
		Spec: "Web IDL", SpecDate: "2026-04-10",
		PrimitiveTypes: []string{
			"boolean", "byte", "octet", "short", "unsigned short",
			"long", "unsigned long", "long long", "unsigned long long",
			"float", "double", "unrestricted float", "unrestricted double",
		},
		StringTypes:  []string{"DOMString", "ByteString", "USVString"},
		ObjectTypes:  []string{"object", "Promise", "FrozenArray", "sequence", "record", "ObservableArray"},
		SpecialTypes: []string{"any", "undefined", "symbol", "ArrayBuffer", "DataView", "Int8Array", "Uint8Array", "Int16Array", "Uint16Array", "Int32Array", "Uint32Array", "BigInt64Array", "BigUint64Array", "Float32Array", "Float64Array", "ArrayBufferView", "BufferSource"},
		ExtendedAttrs: []ExtAttr{
			{"Clamp", "Clamp integer to valid range"},
			{"EnforceRange", "Enforce valid range, throw on overflow"},
			{"LegacyNullToEmptyString", "Convert null to empty string"},
			{"LegacyLenientThis", "Don't throw on wrong this"},
			{"SameObject", "Attribute always returns same object"},
			{"PutForwards", "Forward set to property of returned object"},
			{"Unscopeable", "Exclude from with-scope"},
			{"LegacyUnforgeable", "Cannot be redefined"},
			{"LegacyWindowAlias", "Available under alternate name on Window"},
			{"LegacyNamespace", "Non-standard namespace"},
			{"LegacyFactoryFunction", "Constructor available on Window"},
			{"LegacyArrayClass", "Indexed property getter returns array"},
			{"CEReactions", "Run custom element reactions"},
			{"Exposed", "Expose to specified global scopes"},
			{"SecureContext", "Only expose in secure contexts"},
			{"HTMLConstructor", "Special constructor for custom elements"},
			{"WebGLHandlesContextLoss", "Handle WebGL context loss"},
		},
		CallbackRules: []CallbackRule{
			{"Call", "Called as a function"},
			{"Construct", "Called as a constructor"},
			{"LegacyCallWith", "Called with specific arguments"},
		},
		NullBehavior: []NullBehavior{
			{"Default", "null converts to null"},
			{"LegacyNullToEmptyString", "null converts to empty string"},
			{"TreatNullAs", "Custom null behavior"},
		},
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(catalog)
}
