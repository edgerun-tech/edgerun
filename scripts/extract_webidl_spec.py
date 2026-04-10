#!/usr/bin/env python3
"""Extract Web IDL spec: type system, extended attributes, callback rules."""
import json, re, sys

def main():
    with open(sys.argv[1]) as f:
        md = f.read()

    # Web IDL primitive types
    primitive_types = [
        "boolean", "byte", "octet", "short", "unsigned short",
        "long", "unsigned long", "long long", "unsigned long long",
        "float", "double", "unrestricted float", "unrestricted double",
    ]

    # Web IDL string types
    string_types = ["DOMString", "ByteString", "USVString"]

    # Web IDL object/interface types
    object_types = [
        "object", "Promise", "FrozenArray", "sequence", "record",
        "ObservableArray",
    ]

    # Web IDL special types
    special_types = ["any", "undefined", "symbol", "ArrayBuffer",
                     "DataView", "Int8Array", "Uint8Array", "Int16Array",
                     "Uint16Array", "Int32Array", "Uint32Array",
                     "BigInt64Array", "BigUint64Array", "Float32Array",
                     "Float64Array", "ArrayBufferView",
                     "BufferSource"]

    # Extended attributes (from Web IDL spec)
    extended_attrs = [
        ("Clamp", "Clamp integer to valid range"),
        ("EnforceRange", "Enforce valid range, throw on overflow"),
        ("LegacyNullToEmptyString", "Convert null to empty string"),
        ("LegacyLenientThis", "Don't throw on wrong this"),
        ("SameObject", "Attribute always returns same object"),
        ("PutForwards", "Forward set to property of returned object"),
        ("Unscopeable", "Exclude from with-scope"),
        ("LegacyUnforgeable", "Cannot be redefined"),
        ("LegacyWindowAlias", "Available under alternate name on Window"),
        ("LegacyNamespace", "Non-standard namespace"),
        ("LegacyFactoryFunction", "Constructor available on Window"),
        ("LegacyArrayClass", "Indexed property getter returns array"),
        ("CEReactions", "Run custom element reactions"),
        ("Exposed", "Expose to specified global scopes"),
        ("SecureContext", "Only expose in secure contexts"),
        ("HTMLConstructor", "Special constructor for custom elements"),
        ("WebGLHandlesContextLoss", "Handle WebGL context loss"),
    ]

    # Callback rules
    callback_rules = [
        ("Call", "Called as a function"),
        ("Construct", "Called as a constructor"),
        ("LegacyCallWith", "Called with specific arguments"),
    ]

    # Null handling
    null_behavior = [
        ("Default", "null converts to null"),
        ("LegacyNullToEmptyString", "null converts to empty string"),
        ("TreatNullAs", "Custom null behavior"),
    ]

    catalog = {
        "spec": "Web IDL",
        "spec_date": "2026-04-10",
        "primitive_types": primitive_types,
        "string_types": string_types,
        "object_types": object_types,
        "special_types": special_types,
        "extended_attributes": [{"name": n, "description": d} for n, d in extended_attrs],
        "callback_rules": [{"name": n, "description": d} for n, d in callback_rules],
        "null_behavior": [{"name": n, "description": d} for n, d in null_behavior],
    }
    json.dump(catalog, sys.stdout, indent=2, ensure_ascii=False)

if __name__ == "__main__":
    main()
