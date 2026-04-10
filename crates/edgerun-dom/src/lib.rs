//! DOM Tree types — generated from WHATWG DOM Living Standard via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod dom {
            include!("gen/edgerun.v0.dom.rs");
        }
    }
}

pub use edgerun::v0::dom::*;
