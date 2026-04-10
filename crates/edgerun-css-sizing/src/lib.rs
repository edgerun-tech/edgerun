//! CSS Sizing types — generated via protobuf.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod sizing {
                include!("gen/edgerun.v0.css.sizing.rs");
            }
        }
    }
}

pub use edgerun::v0::css::sizing::*;
