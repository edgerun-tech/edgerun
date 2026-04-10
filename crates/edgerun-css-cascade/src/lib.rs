//! CSS Cascade types — generated via protobuf.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod cascade {
                include!("gen/edgerun.v0.css.cascade.rs");
            }
        }
    }
}

pub use edgerun::v0::css::cascade::*;
