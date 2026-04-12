//! CSS Containment types — generated from CSS Containment Level 3 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod contain {
                include!("gen/edgerun.v0.css.contain.rs");
            }
        }
    }
}

pub use edgerun::v0::css::contain::*;
