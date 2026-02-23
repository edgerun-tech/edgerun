// SPDX-License-Identifier: GPL-2.0-only
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/edgerun.os.v1.rs"));
}

pub use proto::*;
