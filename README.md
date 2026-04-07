# Rust workspace

This directory contains a multi-crate Rust workspace for the edgerun protocol surface and capability backends.

The workspace is organized into crate groups by ownership/risk boundary:

- **Protocol core**
  - `edgerun-core`
  - `edgerun-proto`
  - `edgerun-json`
  - `edgerun-capabilities`
  - `edgerun-machine-report`
  - `edgerun-remote-capability`

- **Media & input abstractions**
  - `edgerun-input`, `edgerun-evdev-input`
  - `edgerun-microphone`, `edgerun-alsa-microphone`
  - `edgerun-speaker`, `edgerun-alsa-speaker`
  - `edgerun-audio-calibration`, `edgerun-camera-biometrics`
  - `edgerun-v4l2-camera`
  - `edgerun-display`, `edgerun-drm-display`

- **Security + identity adapters**
  - `edgerun-fingerprint`, `edgerun-goodix-fingerprint`, `edgerun-biometrics`
  - `edgerun-tpm`, `edgerun-yubikey`
  - `edgerun-android-keystore`, `edgerun-hardware-signing`

- **Connectivity/backends**
  - `edgerun-bluetooth`, `edgerun-mgmt-bluetooth`
  - `edgerun-wifi`, `edgerun-linux-wifi`
  - `edgerun-network-interface`, `edgerun-linux-netif`
  - `edgerun-usb`, `edgerun-linux-usb`
  - `edgerun-pci`, `edgerun-linux-pci`
  - `edgerun-nfc`, `edgerun-linux-nfc`
  - `edgerun-npu`, `edgerun-linux-npu`

- **Application / experiment crates**
  - `edgerun-amd-xdna`

## Commands

```bash
cargo check --workspace
```

## Useful quick commands

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

```bash
# Inventory CLI examples
cargo run -q -p edgerun-linux-pci --bin linux-pci-tool -- list
cargo run -q -p edgerun-linux-usb --bin linux-usb-tool -- list
cargo run -q -p edgerun-linux-netif --bin linux-netif-tool -- list
cargo run -q -p edgerun-linux-wifi --bin linux-wifi-tool -- list
```
