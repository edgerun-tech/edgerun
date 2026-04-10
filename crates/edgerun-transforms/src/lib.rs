//! CSS Transform types — generated from CSS Transforms Level 1 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod transforms {
                include!("gen/edgerun.v0.css.transforms.rs");
            }
        }
    }
}

pub use edgerun::v0::css::transforms::*;
