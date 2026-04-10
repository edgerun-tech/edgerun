//! ECMAScript Built-in Object Registry — generated from ECMA-262.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsObjectId {
    Unspecified = 0,
    Object = 1,
    Function = 2,
    Boolean = 3,
    Symbol = 4,
    Error = 5,
    Number = 6,
    Bigint = 7,
    Date = 8,
    String = 9,
    Regexp = 10,
    Array = 11,
    Uint8array = 12,
    Map = 13,
    Set = 14,
    Weakmap = 15,
    Weakset = 16,
    Arraybuffer = 17,
    Sharedarraybuffer = 18,
    Dataview = 19,
    Weakref = 20,
    Finalizationregistry = 21,
    Iterator = 22,
    Promise = 23,
    Aggregateerror = 24,
    Generatorfunction = 25,
    Asyncgeneratorfunction = 26,
    Asyncfunction = 27,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsMemberKind { Method, Getter, Setter, Data }

#[derive(Debug, Clone)]
pub struct JsMemberDef {
    pub name: &'static str,
    pub kind: JsMemberKind,
    pub param_count: u8,
    pub is_prototype: bool,
}

#[derive(Debug, Clone)]
pub struct JsObjectDef {
    pub id: JsObjectId,
    pub name: &'static str,
    pub members: &'static [JsMemberDef],
}

pub struct JsObjectRegistry;
pub const JSO_OBJECT: JsObjectDef = JsObjectDef {
    id: JsObjectId::Object,
    name: "Object",
    members: &[JsMemberDef { name: "hasOwnProperty", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "isPrototypeOf", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "propertyIsEnumerable", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toLocaleString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "valueOf", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_FUNCTION: JsObjectDef = JsObjectDef {
    id: JsObjectId::Function,
    name: "Function",
    members: &[JsMemberDef { name: "apply", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "bind", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "call", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_BOOLEAN: JsObjectDef = JsObjectDef {
    id: JsObjectId::Boolean,
    name: "Boolean",
    members: &[JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "valueOf", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_SYMBOL: JsObjectDef = JsObjectDef {
    id: JsObjectId::Symbol,
    name: "Symbol",
    members: &[JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "valueOf", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "asyncIterator", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "hasInstance", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "isConcatSpreadable", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "iterator", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "match", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "matchAll", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "replace", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "search", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "species", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "split", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "toPrimitive", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "toStringTag", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "unscopables", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_ERROR: JsObjectDef = JsObjectDef {
    id: JsObjectId::Error,
    name: "Error",
    members: &[JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }, JsMemberDef { name: "message", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }, JsMemberDef { name: "name", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_NUMBER: JsObjectDef = JsObjectDef {
    id: JsObjectId::Number,
    name: "Number",
    members: &[JsMemberDef { name: "toExponential", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toFixed", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toLocaleString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toPrecision", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "valueOf", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "EPSILON", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "MAX_SAFE_INTEGER", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "MAX_VALUE", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "MIN_SAFE_INTEGER", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "MIN_VALUE", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "NaN", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "NEGATIVE_INFINITY", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "POSITIVE_INFINITY", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_BIGINT: JsObjectDef = JsObjectDef {
    id: JsObjectId::Bigint,
    name: "BigInt",
    members: &[JsMemberDef { name: "toLocaleString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "valueOf", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_DATE: JsObjectDef = JsObjectDef {
    id: JsObjectId::Date,
    name: "Date",
    members: &[JsMemberDef { name: "getDate", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getDay", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getFullYear", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getHours", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getMilliseconds", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getMinutes", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getMonth", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getSeconds", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getTime", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getTimezoneOffset", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCDate", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCDay", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCFullYear", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCHours", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCMilliseconds", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCMinutes", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCMonth", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getUTCSeconds", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "setDate", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setFullYear", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setHours", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setMilliseconds", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setMinutes", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setMonth", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setSeconds", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setTime", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setUTCDate", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setUTCFullYear", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setUTCHours", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setUTCMilliseconds", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setUTCMinutes", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setUTCMonth", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setUTCSeconds", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toDateString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toISOString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toJSON", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toLocaleDateString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toLocaleString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toLocaleTimeString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toTimeString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toUTCString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "valueOf", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "getYear", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "setYear", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toGMTString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_STRING: JsObjectDef = JsObjectDef {
    id: JsObjectId::String,
    name: "String",
    members: &[JsMemberDef { name: "at", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "charAt", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "charCodeAt", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "codePointAt", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "concat", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "endsWith", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "includes", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "indexOf", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "isWellFormed", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "lastIndexOf", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "localeCompare", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "match", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "matchAll", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "normalize", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "padEnd", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "padStart", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "repeat", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "replace", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "replaceAll", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "search", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "slice", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "split", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "startsWith", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "substring", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "toLocaleLowerCase", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toLocaleUpperCase", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toLowerCase", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toUpperCase", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toWellFormed", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "trim", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "trimEnd", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "trimStart", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "valueOf", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "substr", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "anchor", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "big", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "blink", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "bold", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "fixed", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "fontcolor", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "fontsize", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "italics", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "link", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "small", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "strike", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "sub", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "sup", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "trimLeft", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "trimRight", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_REGEXP: JsObjectDef = JsObjectDef {
    id: JsObjectId::Regexp,
    name: "RegExp",
    members: &[JsMemberDef { name: "exec", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "test", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "compile", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_ARRAY: JsObjectDef = JsObjectDef {
    id: JsObjectId::Array,
    name: "Array",
    members: &[JsMemberDef { name: "at", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "concat", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "copyWithin", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "entries", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "every", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "fill", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "filter", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "find", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "findIndex", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "findLast", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "findLastIndex", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "flat", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "flatMap", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "forEach", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "includes", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "indexOf", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "join", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "keys", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "lastIndexOf", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "map", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "pop", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "push", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "reduce", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "reduceRight", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "reverse", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "shift", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "slice", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "some", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "sort", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "splice", kind: JsMemberKind::Method, param_count: 3, is_prototype: true }, JsMemberDef { name: "toLocaleString", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toReversed", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "toSorted", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toSpliced", kind: JsMemberKind::Method, param_count: 3, is_prototype: true }, JsMemberDef { name: "toString", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "unshift", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "values", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "with", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_UINT8ARRAY: JsObjectDef = JsObjectDef {
    id: JsObjectId::Uint8array,
    name: "Uint8Array",
    members: &[JsMemberDef { name: "setFromBase64", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setFromHex", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toBase64", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toHex", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }],
};

pub const JSO_MAP: JsObjectDef = JsObjectDef {
    id: JsObjectId::Map,
    name: "Map",
    members: &[JsMemberDef { name: "clear", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "delete", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "entries", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "forEach", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "get", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getOrInsert", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "getOrInsertComputed", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "has", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "keys", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "set", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "values", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_SET: JsObjectDef = JsObjectDef {
    id: JsObjectId::Set,
    name: "Set",
    members: &[JsMemberDef { name: "add", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "clear", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "delete", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "difference", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "entries", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "forEach", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "has", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "intersection", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "isDisjointFrom", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "isSubsetOf", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "isSupersetOf", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "keys", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "symmetricDifference", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "union", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "values", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_WEAKMAP: JsObjectDef = JsObjectDef {
    id: JsObjectId::Weakmap,
    name: "WeakMap",
    members: &[JsMemberDef { name: "delete", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "get", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getOrInsert", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "getOrInsertComputed", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "has", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "set", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_WEAKSET: JsObjectDef = JsObjectDef {
    id: JsObjectId::Weakset,
    name: "WeakSet",
    members: &[JsMemberDef { name: "add", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "delete", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "has", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_ARRAYBUFFER: JsObjectDef = JsObjectDef {
    id: JsObjectId::Arraybuffer,
    name: "ArrayBuffer",
    members: &[JsMemberDef { name: "resize", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "slice", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "transfer", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "transferToFixedLength", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_SHAREDARRAYBUFFER: JsObjectDef = JsObjectDef {
    id: JsObjectId::Sharedarraybuffer,
    name: "SharedArrayBuffer",
    members: &[JsMemberDef { name: "grow", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "slice", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_DATAVIEW: JsObjectDef = JsObjectDef {
    id: JsObjectId::Dataview,
    name: "DataView",
    members: &[JsMemberDef { name: "getBigInt64", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getBigUint64", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getFloat16", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getFloat32", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getFloat64", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getInt8", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getInt16", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getInt32", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getUint8", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getUint16", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "getUint32", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "setBigInt64", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setBigUint64", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setFloat16", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setFloat32", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setFloat64", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setInt8", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setInt16", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setInt32", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setUint8", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setUint16", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "setUint32", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_WEAKREF: JsObjectDef = JsObjectDef {
    id: JsObjectId::Weakref,
    name: "WeakRef",
    members: &[JsMemberDef { name: "deref", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_FINALIZATIONREGISTRY: JsObjectDef = JsObjectDef {
    id: JsObjectId::Finalizationregistry,
    name: "FinalizationRegistry",
    members: &[JsMemberDef { name: "register", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "unregister", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_ITERATOR: JsObjectDef = JsObjectDef {
    id: JsObjectId::Iterator,
    name: "Iterator",
    members: &[JsMemberDef { name: "drop", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "every", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "filter", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "find", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "flatMap", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "forEach", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "map", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "reduce", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "some", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "take", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "toArray", kind: JsMemberKind::Method, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_PROMISE: JsObjectDef = JsObjectDef {
    id: JsObjectId::Promise,
    name: "Promise",
    members: &[JsMemberDef { name: "catch", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "finally", kind: JsMemberKind::Method, param_count: 1, is_prototype: true }, JsMemberDef { name: "then", kind: JsMemberKind::Method, param_count: 2, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_AGGREGATEERROR: JsObjectDef = JsObjectDef {
    id: JsObjectId::Aggregateerror,
    name: "AggregateError",
    members: &[JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }, JsMemberDef { name: "message", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }, JsMemberDef { name: "name", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_GENERATORFUNCTION: JsObjectDef = JsObjectDef {
    id: JsObjectId::Generatorfunction,
    name: "GeneratorFunction",
    members: &[JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_ASYNCGENERATORFUNCTION: JsObjectDef = JsObjectDef {
    id: JsObjectId::Asyncgeneratorfunction,
    name: "AsyncGeneratorFunction",
    members: &[JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }, JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};

pub const JSO_ASYNCFUNCTION: JsObjectDef = JsObjectDef {
    id: JsObjectId::Asyncfunction,
    name: "AsyncFunction",
    members: &[JsMemberDef { name: "prototype", kind: JsMemberKind::Data, param_count: 0, is_prototype: false }, JsMemberDef { name: "constructor", kind: JsMemberKind::Data, param_count: 0, is_prototype: true }],
};


impl JsObjectRegistry {
    pub fn by_name(name: &str) -> Option<&'static JsObjectDef> {
        match name {
            "Object" => Some(&JSO_OBJECT),
            "Function" => Some(&JSO_FUNCTION),
            "Boolean" => Some(&JSO_BOOLEAN),
            "Symbol" => Some(&JSO_SYMBOL),
            "Error" => Some(&JSO_ERROR),
            "Number" => Some(&JSO_NUMBER),
            "BigInt" => Some(&JSO_BIGINT),
            "Date" => Some(&JSO_DATE),
            "String" => Some(&JSO_STRING),
            "RegExp" => Some(&JSO_REGEXP),
            "Array" => Some(&JSO_ARRAY),
            "Uint8Array" => Some(&JSO_UINT8ARRAY),
            "Map" => Some(&JSO_MAP),
            "Set" => Some(&JSO_SET),
            "WeakMap" => Some(&JSO_WEAKMAP),
            "WeakSet" => Some(&JSO_WEAKSET),
            "ArrayBuffer" => Some(&JSO_ARRAYBUFFER),
            "SharedArrayBuffer" => Some(&JSO_SHAREDARRAYBUFFER),
            "DataView" => Some(&JSO_DATAVIEW),
            "WeakRef" => Some(&JSO_WEAKREF),
            "FinalizationRegistry" => Some(&JSO_FINALIZATIONREGISTRY),
            "Iterator" => Some(&JSO_ITERATOR),
            "Promise" => Some(&JSO_PROMISE),
            "AggregateError" => Some(&JSO_AGGREGATEERROR),
            "GeneratorFunction" => Some(&JSO_GENERATORFUNCTION),
            "AsyncGeneratorFunction" => Some(&JSO_ASYNCGENERATORFUNCTION),
            "AsyncFunction" => Some(&JSO_ASYNCFUNCTION),
            _ => None,
        }
    }
}