//! CSS Animation types — generated from CSS Animations Level 1 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod animations {
                include!("gen/edgerun.v0.css.animations.rs");
            }
        }
    }
}

pub use edgerun::v0::css::animations::*;
