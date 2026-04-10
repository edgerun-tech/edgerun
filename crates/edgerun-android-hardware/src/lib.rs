//! Android NDK hardware capability providers for edgerun nodes.
//!
//! Uses direct FFI to Android NDK shared libraries where C APIs exist,
//! and sysfs for power/battery info. JNI is used for BiometricPrompt and
//! LocationManager where no C API exists.
//!
//! Enable `feature = "android-real"` to get real implementations.
//! Otherwise, stub implementations are available for cross-compilation.

mod input;
mod audio;
mod camera;
mod sensors;
mod display;
mod biometric;
mod location;
mod power;

// Re-export all providers
pub use input::{AndroidInputProvider, AInputDevice};
pub use audio::{AndroidAudioInputProvider, AndroidAudioOutputProvider, AAudioStream};
pub use camera::{AndroidCameraProvider, Camera2Session};
pub use sensors::{AndroidSensorProvider, ASensorManagerWrapper as ASensor};
pub use display::{AndroidDisplayProvider, ANativeWindowSurface};
pub use biometric::AndroidBiometricProvider;
pub use location::AndroidLocationProvider;
pub use power::{AndroidPowerProvider, BatteryInfo};
