//! edgerun-html — HTML element, attribute, and DOM interface definitions.
//!
//! Types are generated from protobuf definitions derived from the
//! WHATWG HTML Living Standard.
//!
//! Pipeline:
//!   1. `scripts/extract_html_spec.py` — parses spec markdown → catalog JSON
//!   2. `scripts/generate_html_proto.py` — catalog JSON → `.proto` files
//!   3. `buf generate` — `.proto` files → Rust source in `src/gen/`

pub mod edgerun {
    pub mod v0 {
        pub mod html {
            pub mod elements {
                include!("gen/edgerun.v0.html.elements.rs");
            }
            pub mod attributes {
                include!("gen/edgerun.v0.html.attributes.rs");
            }
        }
    }
}

pub use edgerun::v0::html::elements::*;
pub use edgerun::v0::html::attributes::*;
