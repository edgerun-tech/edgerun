//! Android NDK hardware capability providers for edgerun nodes.
//!
//! Uses direct FFI to Android NDK shared libraries where C APIs exist,
//! and sysfs for power/battery info. JNI is used for BiometricPrompt and
//! LocationManager where no C API exists.
//!
//! Enable `feature = "android-real"` to get real implementations.
//! Otherwise, stub implementations are available for cross-compilation.

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
