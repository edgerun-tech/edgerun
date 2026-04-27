//! Android NDK hardware capability providers for edgerun nodes.
//!
//! Uses direct FFI to Android NDK shared libraries where C APIs exist,
//! and sysfs for power/battery info. Capabilities without a stable C/sysfs
//! surface report unsupported from the real backend.
//!
//! Enable `feature = "android-real"` to get real implementations.
//! Otherwise, stub implementations are available for cross-compilation.

#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
pub mod prelude {
    pub mod rust_2024 {
        pub use alloc::string::{String, ToString};
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2024::*;
    }
}

#[cfg(target_os = "none")]
pub mod ffi {
    pub use core::ffi::*;
}

mod audio;
mod biometric;
mod camera;
mod display;
mod input;
mod location;
mod power;
mod sensors;

// Re-export all providers
pub use audio::{AAudioStream, AndroidAudioInputProvider, AndroidAudioOutputProvider};
pub use biometric::AndroidBiometricProvider;
pub use camera::{AndroidCameraProvider, Camera2Session};
pub use display::{ANativeWindowSurface, AndroidDisplayProvider};
pub use input::{AInputDevice, AndroidInputProvider};
pub use location::AndroidLocationProvider;
pub use power::{AndroidPowerProvider, BatteryInfo};
pub use sensors::{ASensorManagerWrapper as ASensor, AndroidSensorProvider};
