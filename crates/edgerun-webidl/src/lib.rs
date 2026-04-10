//! Web IDL type system — from Web IDL specification.
//! DO NOT EDIT. Regenerate with: scripts/generate_web_platform2.py
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{boxed::Box, string::String, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    /// boolean
    Boolean,
    /// byte
    Byte,
    /// octet
    Octet,
    /// short
    Short,
    /// unsigned short
    UnsignedShort,
    /// long
    Long,
    /// unsigned long
    UnsignedLong,
    /// long long
    LongLong,
    /// unsigned long long
    UnsignedLongLong,
    /// float
    Float,
    /// double
    Double,
    /// unrestricted float
    UnrestrictedFloat,
    /// unrestricted double
    UnrestrictedDouble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringType {
    Domstring,
    Bytestring,
    Usvstring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Object,
    Promise,
    Frozenarray,
    Sequence,
    Record,
    Observablearray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialType {
    Any,
    Undefined,
    Symbol,
    Arraybuffer,
    Dataview,
    Int8array,
    Uint8array,
    Int16array,
    Uint16array,
    Int32array,
    Uint32array,
    Bigint64array,
    Biguint64array,
    Float32array,
    Float64array,
    Arraybufferview,
    Buffersource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedAttr {
    /// Clamp: Clamp integer to valid range
    Clamp,
    /// EnforceRange: Enforce valid range, throw on overflow
    Enforcerange,
    /// LegacyNullToEmptyString: Convert null to empty string
    Legacynulltoemptystring,
    /// LegacyLenientThis: Don't throw on wrong this
    Legacylenientthis,
    /// SameObject: Attribute always returns same object
    Sameobject,
    /// PutForwards: Forward set to property of returned object
    Putforwards,
    /// Unscopeable: Exclude from with-scope
    Unscopeable,
    /// LegacyUnforgeable: Cannot be redefined
    Legacyunforgeable,
    /// LegacyWindowAlias: Available under alternate name on Window
    Legacywindowalias,
    /// LegacyNamespace: Non-standard namespace
    Legacynamespace,
    /// LegacyFactoryFunction: Constructor available on Window
    Legacyfactoryfunction,
    /// LegacyArrayClass: Indexed property getter returns array
    Legacyarrayclass,
    /// CEReactions: Run custom element reactions
    Cereactions,
    /// Exposed: Expose to specified global scopes
    Exposed,
    /// SecureContext: Only expose in secure contexts
    Securecontext,
    /// HTMLConstructor: Special constructor for custom elements
    Htmlconstructor,
    /// WebGLHandlesContextLoss: Handle WebGL context loss
    Webglhandlescontextloss,
}

/// A Web IDL type.
#[derive(Debug, Clone, PartialEq)]
pub enum WebIdlType {
    Primitive(PrimitiveType),
    String(StringType),
    Object(ObjectType),
    Special(SpecialType),
    Nullable(Box<WebIdlType>),
    FrozenArray(Box<WebIdlType>),
    Sequence(Box<WebIdlType>),
    Interface(String),
}