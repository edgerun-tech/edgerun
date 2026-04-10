//! edgerun-selectors — CSS Selectors Level 4 types.
//! Generated from W3C Selectors Level 4 specification.
#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::{string::String, vec::Vec};

pub mod pseudo_classes;
pub mod pseudo_elements;
pub mod combinators;
pub mod attr_selectors;
pub mod specificity;
