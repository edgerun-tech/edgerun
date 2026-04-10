//! CSS Media Query types — generated from Media Queries Level 4 via protobuf.
//! DO NOT EDIT. Regenerate proto then run `buf generate`.
#![cfg_attr(not(test), no_std)]

pub mod edgerun {
    pub mod v0 {
        pub mod css {
            pub mod media_queries {
                include!("gen/edgerun.v0.css.media_queries.rs");
            }
        }
    }
}

pub use edgerun::v0::css::media_queries::*;
