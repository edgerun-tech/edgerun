//! CSS Font types — generated from CSS Fonts Level 4 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod fonts {
                include!("gen/edgerun.v0.css.fonts.rs");
            }
        }
    }
}

pub use edgerun::v0::css::fonts::*;
