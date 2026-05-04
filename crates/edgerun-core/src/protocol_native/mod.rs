//! Native Edgerun protocol types.
//!
//! This module is generated from the old protobuf-shaped Rust files, but it is
//! plain Rust inside edgerun-core. There is no edgerun-proto crate and no prost
//! derive in this module.

extern crate alloc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Timestamp {
    pub seconds: i64,
    pub nanos: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Duration {
    pub seconds: i64,
    pub nanos: i32,
}

#[path = "gen/edgerun.v0.access.rs"]
pub mod access;

#[path = "gen/edgerun.v0.app.rs"]
pub mod app;

#[path = "gen/edgerun.v0.appabi.rs"]
pub mod appabi;

#[path = "gen/edgerun.v0.capability.rs"]
pub mod capability;

#[path = "gen/edgerun.v0.capability_runtime.rs"]
pub mod capability_runtime;

#[path = "gen/edgerun.v0.common.rs"]
pub mod common;

#[path = "gen/edgerun.v0.identity.rs"]
pub mod identity;

#[path = "gen/edgerun.v0.network.rs"]
pub mod network;

#[path = "gen/edgerun.v0.object.rs"]
pub mod object;

#[path = "gen/edgerun.v0.server_resources.rs"]
pub mod server_resources;

#[path = "gen/edgerun.v0.stream.rs"]
pub mod stream;

#[path = "gen/edgerun.v0.trust.rs"]
pub mod trust;

#[path = "gen/edgerun.v0.ui.rs"]
pub mod ui;

#[path = "gen/edgerun.wallet.v0.rs"]
pub mod edgerun_wallet_v0;

pub use access::*;
pub use app::*;
pub use appabi::*;
pub use capability::*;
pub use capability_runtime::*;
pub use common::*;
pub use edgerun_wallet_v0::*;
pub use identity::*;
pub use network::*;
pub use object::*;
pub use server_resources::*;
pub use stream::*;
pub use trust::*;
pub use ui::*;
