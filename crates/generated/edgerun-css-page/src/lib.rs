//! CSS Page types — generated via protobuf.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod page {
                include!("gen/edgerun.v0.css.page.rs");
            }
        }
    }
}

pub use edgerun::v0::css::page::*;
