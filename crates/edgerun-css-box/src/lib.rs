//! CSS Box types — generated via protobuf.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod r#box {
                include!("gen/edgerun.v0.css.box.rs");
            }
        }
    }
}

pub use edgerun::v0::css::r#box::*;
