//! CSS Transition types — generated from CSS Transitions Level 1 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod transitions {
                include!("gen/edgerun.v0.css.transitions.rs");
            }
        }
    }
}

pub use edgerun::v0::css::transitions::*;
