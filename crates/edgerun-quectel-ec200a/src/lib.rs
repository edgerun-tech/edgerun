//! Quectel EC200A Series LTE Module Hardware Driver
//!
//! This crate provides hardware-level driver support for the Quectel EC200A series
//! LTE Cat 4 wireless communication modules.
//!
//! # Module Variants
//! - EC200A-CN: China variant
//! - EC200A-AU: Australia variant  
//! - EC200A-EU: Europe variant
//! - EC200A-EL: LTE variant
//!
//! # Key Features
//! - LTE Cat 4: 150 Mbps downlink, 50 Mbps uplink
//! - WCDMA/GSM fallback
//! - LCC 80-pin / LGA 64-pin packaging
//! - DTA (Direct Terminal Access) network support via AT commands

#![no_std]

extern crate alloc;

pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

pub mod string {
    pub use alloc::string::{String, ToString};
}

pub mod vec {
    pub use alloc::vec::Vec;
}

pub use alloc::format;
pub use core::{default, option, result};

use core::marker::PhantomData;

pub mod dta;
pub mod interfaces;
pub mod pins;
pub mod power;

pub use dta::{
    Config, check_dta_status, configure_dta, get_imsi, get_modem_info, get_network_operator,
    get_signal_quality, set_radio_function,
};
pub use interfaces::*;
pub use pins::*;
pub use power::*;

/// EC200A module series identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Model {
    CN,
    AU,
    EU,
    EL,
}

impl Model {
    pub fn as_str(self) -> &'static str {
        match self {
            Model::CN => "EC200A-CN",
            Model::AU => "EC200A-AU",
            Model::EU => "EC200A-EU",
            Model::EL => "EC200A-EL",
        }
    }
}

/// LTE band categories supported by the module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Band {
    /// LTE-FDD bands
    Fdd(u8),
    /// LTE-TDD bands
    Tdd(u8),
    /// WCDMA bands
    Wcdma(u8),
    /// GSM bands
    Gsm(u8),
}

impl Band {
    pub fn lte_fdd(band: u8) -> Self {
        Self::Fdd(band)
    }

    pub fn lte_tdd(band: u8) -> Self {
        Self::Tdd(band)
    }
}

/// Network type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[derive(Default)]
pub enum NetworkType {
    LTE = 1,
    Wcdma = 2,
    Gsm = 3,
    #[default]
    Auto = 4,
}

/// Module operating state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[derive(Default)]
pub enum State {
    #[default]
    Unknown = 0,
    Off = 1,
    On = 2,
    Sleep = 3,
    NetworkSearch = 4,
    Registered = 5,
    Connected = 6,
}

/// DTA (Direct Terminal Access) network configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[derive(Default)]
pub enum DtaNetwork {
    #[default]
    Disabled = 0,
    Enabled = 1,
}

/// EC200A module instance
pub struct Ec200a<M: ModelVariant> {
    pub _model: PhantomData<M>,
}

pub trait ModelVariant {}
impl ModelVariant for Model {}
