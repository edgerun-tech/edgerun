# Android Hardware Support for edgerun

This document describes the Android hardware backend implementation for edgerun nodes.

## Architecture

```
edgerund (node daemon)
├── HardwareInventory::discover()
│   ├── On Linux: Linux drivers (V4L2, ALSA, DRM, evdev, sysfs)
│   └── On Android: NDK FFI + sysfs (see below)
│
├── Signer (from config)
│   ├── software: In-memory ECDSA P-256
│   ├── tpm: TPM 2.0 via /dev/tpmrm0
│   ├── yubikey: YubiKey PIV via PC/SC
│   └── android-keystore: Android Keystore via JNI
│
└── MeshDaemon
    ├── Session encryption (ECDH + AES-GCM)
    ├── Frame signing (SHA-256 + ECDSA)
    └── Command handler → store task
```

## Hardware Providers

| Provider | NDK Library | API | Status |
|----------|------------|-----|--------|
| **Input** | `libinput.so` | `AInputQueue`, `AInputEvent` | ✅ Real (dlopen) |
| **Audio (mic)** | `libaaudio.so` | `AAudioStream` (input) | ✅ Real (dlopen) |
| **Audio (speaker)** | `libaaudio.so` | `AAudioStream` (output) | ✅ Real (dlopen) |
| **Camera** | `libcamera2_ndk.so` | `ACameraManager` | ✅ Real (dlopen) |
| **Sensors** | `libsensor.so` | `ASensorManager` | ✅ Real (dlopen) |
| **Display** | `libnative_window.so` | `ANativeWindow` | ✅ Real (linked) |
| **Biometric** | `android.hardware.biometrics` | `BiometricManager` (JNI) | ✅ Real (JNI) |
| **Location** | sysfs | `/sys/class/gps/` | ✅ Real (sysfs) |
| **Power** | sysfs | `/sys/class/power_supply/` | ✅ Real (sysfs) |
| **Keystore** | `android.security.keystore` | Java API (JNI) | ✅ Real (JNI) |

## Building for Android

### Prerequisites

1. **Android NDK** (r27 or later)
   ```bash
   export ANDROID_NDK_ROOT=$HOME/Android/Sdk/ndk/27.0.12077973
   ```

2. **Android SDK** with system image
   ```bash
   $ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager \
       "system-images;android-34;google_apis;x86_64"
   ```

3. **Rust Android targets**
   ```bash
   rustup target add aarch64-linux-android
   rustup target add x86_64-linux-android
   ```

### Build Commands

```bash
# Build release binary for ARM64 (real hardware)
cargo build --target aarch64-linux-android --release --features android-real

# Build for x86_64 emulator
cargo build --target x86_64-linux-android --release --features android-real

# Run tests (uses stub implementations)
cargo test --package edgerun-android-hardware
cargo test --package edgerun-android-keystore
```

### Using cargo-ndk (recommended)

```bash
cargo install cargo-ndk

# Build for Android with automatic NDK linking
cargo ndk --target aarch64-linux-android --platform 34 -- \
    build --release --features android-real
```

## Running on Android Emulator

```bash
# Start the emulator (see scripts/setup-android-emulator.sh)
./scripts/setup-android-emulator.sh start

# Build and push the binary
./scripts/setup-android-emulator.sh push

# Run on the emulator
./scripts/setup-android-emulator.sh shell
/data/local/tmp/edgerund init --config /data/local/tmp/node.yaml --software
/data/local/tmp/edgerund run --config /data/local/tmp/node.yaml
```

## Testing

### Unit Tests (run on host, use stubs)

```bash
# All Android hardware provider tests (20 tests)
cargo test --package edgerun-android-hardware

# Biometric-specific tests (15 tests)
cargo test --package edgerun-android-hardware --test android_biometric

# Integration tests (17 tests)
cargo test --package edgerun-android-hardware --test android_integration

# Keystore tests (3 tests)
cargo test --package edgerun-android-keystore

# All Android tests (55 tests total)
cargo test --package edgerun-android-hardware --package edgerun-android-keystore
```

### Integration Tests (run on emulator)

```bash
# Run on emulator with real NDK implementations
./scripts/setup-android-emulator.sh test
```

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `android-real` | Enable real NDK/JNI implementations | `ndk-sys`, `jni`, `ndk-context`, `once_cell` |
| `android-hardware` (node) | Enable Android hardware support in the node | `edgerun-android-hardware`, `edgerun-android-keystore` |

Without `android-real`, all providers return stub implementations that compile on any target.

## Implementation Details

### NDK Library Loading

All NDK libraries are loaded at runtime via `dlopen`/`dlsym`:

```rust
static LIB_AAUDIO: OnceCell<AaudioFns> = OnceCell::new();

fn ensure_loaded() -> Result<&'static AaudioFns, CapabilityError> {
    LIB_AAUDIO.get_or_try_init(|| {
        let handle = libc::dlopen(b"libaaudio.so\0".as_ptr() as *const _, libc::RTLD_LAZY);
        // ... load function pointers via dlsym
    })
}
```

This means:
- No compile-time NDK dependency for stub builds
- Libraries are only loaded when actually used
- Graceful degradation if a library is missing

### JNI Integration

The Android Keystore uses JNI to call Java APIs:

```rust
// Initialize JVM context (called once from NativeActivity)
pub fn init_keystore_jvm() -> Result<(), AndroidKeystoreError> {
    let ctx = ndk_context::android_context();
    let vm = jni::JavaVM::from_raw(ctx.vm())?;
    JAVA_VM.set(vm)?;
    Ok(())
}
```

### Biometric Authentication

The biometric provider uses JNI to call Android's `BiometricManager` API:

```rust
// Check biometric availability
let manager = JniBiometricManager::new()?;
if manager.is_available() {
    let modalities = manager.get_available_modalities();
    // [Fingerprint, Face, Iris]
}
```

For actual authentication, the recommended approach on Android is to use
Keystore-bound keys with `setUserAuthenticationRequired(true)`. When you
attempt a cryptographic operation with such a key, the system automatically
shows the biometric prompt. The `request_authentication()` method checks
enrollment status and returns detailed error messages (lockout, no hardware,
etc.).

### Power and Location via sysfs

Battery and GPS data are read directly from the Linux kernel's sysfs interface, which is available on all Android devices:

```rust
// Battery percentage
let level = std::fs::read_to_string("/sys/class/power_supply/battery/capacity")?;

// GPS coordinates (if exposed by kernel)
let lat = std::fs::read_to_string("/sys/class/gps/latitude")?;
```

This avoids the need for JNI calls for these providers.
