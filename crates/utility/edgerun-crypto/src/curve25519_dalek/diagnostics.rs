//! Build time diagnostics

// serial was selected
#[cfg(curve25519_dalek_backend = "serial")]
compile_error!("curve25519_dalek_backend is 'serial'");

// 32 bits target_pointer_width was selected
#[cfg(curve25519_dalek_bits = "32")]
compile_error!("curve25519_dalek_bits is '32'");

// 64 bits target_pointer_width was selected
#[cfg(curve25519_dalek_bits = "64")]
compile_error!("curve25519_dalek_bits is '64'");
