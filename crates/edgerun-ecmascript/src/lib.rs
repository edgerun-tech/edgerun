//! edgerun-ecmascript — ECMAScript built-in object, abstract operation, and global definitions.
//!
//! Types are generated from protobuf definitions derived from ECMA-262.
//!
//! Pipeline:
//!   1. `scripts/extract_ecmascript_spec.py` — parses ECMA-262 markdown → catalog JSON
//!   2. `scripts/generate_ecmascript_proto.py` — catalog JSON → `.proto` files
//!   3. `buf generate` — `.proto` files → Rust source in `src/gen/`

pub mod edgerun {
    pub mod v0 {
        pub mod ecmascript {
            pub mod objects {
                include!("gen/edgerun.v0.ecmascript.objects.rs");
            }
            pub mod abstract_ops {
                include!("gen/edgerun.v0.ecmascript.abstract_ops.rs");
            }
            pub mod globals {
                include!("gen/edgerun.v0.ecmascript.globals.rs");
            }
        }
    }
}

pub use edgerun::v0::ecmascript::objects::*;
pub use edgerun::v0::ecmascript::abstract_ops::*;
pub use edgerun::v0::ecmascript::globals::*;
