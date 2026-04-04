//! Protobuf-generated wire types.
pub mod convert;

pub mod lifegraph {
    pub mod v0 {
        pub mod access {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.access.rs"));
        }
        pub mod capability {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.capability.rs"));
        }
        pub mod capability_runtime {
            include!(concat!(
                env!("OUT_DIR"),
                "/lifegraph.v0.capability_runtime.rs"
            ));
        }
        pub mod common {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.common.rs"));
        }
        pub mod identity {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.identity.rs"));
        }
        pub mod network {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.network.rs"));
        }
        pub mod object {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.object.rs"));
        }
        pub mod stream {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.stream.rs"));
        }
        pub mod trust {
            include!(concat!(env!("OUT_DIR"), "/lifegraph.v0.trust.rs"));
        }
    }
}
