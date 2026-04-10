//! Touch Event types — generated from W3C Touch Events Level 2 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod ui {
            pub mod touch {
                include!("gen/edgerun.v0.ui.touch.rs");
            }
        }
    }
}

pub use edgerun::v0::ui::touch::*;
