//! CSS Display types — generated via protobuf.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod display {
                include!("gen/edgerun.v0.css.display.rs");
            }
        }
    }
}

pub use edgerun::v0::css::display::*;
