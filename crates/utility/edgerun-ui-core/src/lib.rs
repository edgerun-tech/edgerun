#![cfg_attr(not(feature = "std"), no_std)]

//! EdgeRun UI core.
//!
//! The canonical UI implementation builds `GpuScene` command buffers in Rust.
//! Hosts render those buffers and forward input back into Rust. The crate does
//! not expose legacy pixel-buffer painters or software component APIs.

#[cfg(feature = "std")]
pub mod gpu;
pub mod initial_setup;
#[cfg(feature = "lucide-svg-atlas")]
pub mod lucide_svg_atlas_generated;
pub mod record_codec;
#[cfg(feature = "tabler-svg-atlas")]
pub mod tabler_svg_atlas_generated;
