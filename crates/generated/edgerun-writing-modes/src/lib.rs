//! CSS Writing Modes types — generated from CSS Writing Modes Level 4 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod writing_modes {
                include!("gen/edgerun.v0.css.writing_modes.rs");
            }
        }
    }
}

pub use edgerun::v0::css::writing_modes::*;
