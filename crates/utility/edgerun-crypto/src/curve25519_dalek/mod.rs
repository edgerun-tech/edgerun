// -*- mode: rust; -*-
//
// This file is part of curve25519-dalek.
// Copyright (c) 2016-2021 isis lovecruft
// Copyright (c) 2016-2019 Henry de Valence
// See LICENSE for licensing information.
//
// Authors:
// - isis agora lovecruft <isis@patternsinthevoid.net>
// - Henry de Valence <hdevalence@hdevalence.ca>

#![cfg_attr(docsrs, doc(cfg_hide(docsrs)))]
//------------------------------------------------------------------------
// Documentation:
//------------------------------------------------------------------------
//------------------------------------------------------------------------
// Linting:
//------------------------------------------------------------------------
#![allow(unexpected_cfgs)]

//------------------------------------------------------------------------
// External dependencies:
//------------------------------------------------------------------------

#[cfg(feature = "digest")]
pub use crate::digest;

// Internal macros. Must come first!
#[macro_use]
pub(crate) mod macros;

//------------------------------------------------------------------------
// curve25519-dalek public modules
//------------------------------------------------------------------------

// Scalar arithmetic mod l = 2^252 + ..., the order of the Ristretto group
pub mod scalar;

// Point operations on the Montgomery form of Curve25519
pub mod montgomery;

// Point operations on the Edwards form of Curve25519
pub mod edwards;

// Group operations on the Ristretto group
pub mod ristretto;

// Useful constants, like the Ed25519 basepoint
pub mod constants;

// External (and internal) traits.
pub mod traits;

//------------------------------------------------------------------------
// curve25519-dalek internal modules
//------------------------------------------------------------------------

// Finite field arithmetic mod p = 2^255 - 19
pub(crate) mod field;

// Arithmetic backends (using u32, u64, etc) live here
#[cfg(docsrs)]
pub mod backend;
#[cfg(not(docsrs))]
pub(crate) mod backend;

// Generic code for window lookups
pub(crate) mod window;

pub use crate::curve25519_dalek::{
    edwards::EdwardsPoint, montgomery::MontgomeryPoint, ristretto::RistrettoPoint, scalar::Scalar,
};

// Build time diagnostics for validation
#[cfg(curve25519_dalek_diagnostics = "build")]
mod diagnostics;
