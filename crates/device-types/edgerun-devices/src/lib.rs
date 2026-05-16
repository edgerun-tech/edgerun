#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod biometrics;
pub mod bluetooth;
pub mod camera_biometrics;
pub mod cec;
pub mod display;
pub mod fingerprint;
pub mod gpu;
pub mod input;
pub mod microphone;
pub mod network_interface;
pub mod nfc;
pub mod npu;
pub mod pci;
pub mod power;
pub mod quectel_ec200a;
pub mod speaker;
pub mod usb;
pub mod wifi;

pub use biometrics::*;
pub use bluetooth::*;
pub use camera_biometrics::*;
pub use cec::*;
pub use display::*;
pub use fingerprint::*;
pub use gpu::*;
pub use input::*;
pub use microphone::*;
pub use network_interface::*;
pub use nfc::*;
pub use npu::*;
pub use pci::*;
pub use power::*;
pub use quectel_ec200a::*;
pub use speaker::*;
pub use usb::*;
pub use wifi::*;

pub mod prelude {
    pub use crate::biometrics::*;
    pub use crate::bluetooth::*;
    pub use crate::camera_biometrics::*;
    pub use crate::cec::*;
    pub use crate::display::*;
    pub use crate::fingerprint::*;
    pub use crate::gpu::*;
    pub use crate::input::*;
    pub use crate::microphone::*;
    pub use crate::network_interface::*;
    pub use crate::nfc::*;
    pub use crate::npu::*;
    pub use crate::pci::*;
    pub use crate::power::*;
    pub use crate::quectel_ec200a::*;
    pub use crate::speaker::*;
    pub use crate::usb::*;
    pub use crate::wifi::*;
}
