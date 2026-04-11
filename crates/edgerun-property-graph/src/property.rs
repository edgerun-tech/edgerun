//! CSS Property metadata extracted from proto definitions.

use alloc::string::String;
use alloc::vec::Vec;

/// A single CSS property with all its metadata.
#[derive(Clone, Debug)]
pub struct CssProperty {
    /// Property name, e.g. "font-size"
    pub name: String,
    /// Proto enum discriminant
    pub proto_idx: u32,
    /// Source proto file
    pub proto_file: String,
    /// Value syntax from spec comment
    pub value_syntax: String,
    /// Initial value
    pub initial_value: String,
    /// Whether this property inherits from parent
    pub inherits: bool,
    /// Whether changes trigger layout recalculation
    pub affects_layout: bool,
    /// Whether changes trigger paint invalidation
    pub affects_paint: bool,
    /// Whether this property can be animated
    pub animatable: bool,
    /// Computed value description
    pub computed_value: String,
    /// Animation type
    pub animation_type: String,
}
