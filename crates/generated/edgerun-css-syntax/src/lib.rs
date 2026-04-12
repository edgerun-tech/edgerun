//! CSS Syntax types — generated from CSS Syntax Level 3 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod syntax {
                include!("gen/edgerun.v0.css.syntax.rs");
            }
        }
    }
}

pub use edgerun::v0::css::syntax::*;
