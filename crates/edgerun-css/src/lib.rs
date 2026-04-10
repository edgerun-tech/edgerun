//! edgerun-css — CSS property, value type, and at-rule definitions.
//!
//! Types are generated from protobuf definitions derived from W3C CSS
//! specifications.
//!
//! Pipeline:
//!   1. `scripts/extract_css_specs.py` — parses CSS spec markdowns → catalog JSON
//!   2. `scripts/generate_css_proto.py` — catalog JSON → `.proto` files
//!   3. `buf generate` — `.proto` files → Rust source in `src/gen/`

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod properties {
                include!("gen/edgerun.v0.css.properties.rs");
            }
            pub mod value_types {
                include!("gen/edgerun.v0.css.value_types.rs");
            }
            pub mod at_rules {
                include!("gen/edgerun.v0.css.at_rules.rs");
            }
        }
    }
}

pub use edgerun::v0::css::properties::*;
pub use edgerun::v0::css::value_types::*;
pub use edgerun::v0::css::at_rules::*;
