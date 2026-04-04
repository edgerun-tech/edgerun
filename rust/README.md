# Rust workspace

This directory contains a multi-crate Rust workspace for the Lifegraph protocol surface and capability backends.

The workspace is organized into crate groups by ownership/risk boundary:

- **Protocol core**
  - `lifegraph-core`
  - `lifegraph-proto`
  - `lifegraph-json`
  - `lifegraph-capabilities`
  - `lifegraph-machine-report`
  - `lifegraph-remote-capability`

- **Media & input abstractions**
  - `lifegraph-input`, `lifegraph-evdev-input`
  - `lifegraph-microphone`, `lifegraph-alsa-microphone`
  - `lifegraph-speaker`, `lifegraph-alsa-speaker`
  - `lifegraph-audio-calibration`, `lifegraph-camera-biometrics`
  - `lifegraph-v4l2-camera`
  - `lifegraph-display`, `lifegraph-drm-display`

- **Security + identity adapters**
  - `lifegraph-fingerprint`, `lifegraph-goodix-fingerprint`, `lifegraph-biometrics`
  - `lifegraph-tpm`, `lifegraph-yubikey`
  - `lifegraph-android-keystore`, `lifegraph-hardware-signing`

- **Connectivity/backends**
  - `lifegraph-bluetooth`, `lifegraph-mgmt-bluetooth`
  - `lifegraph-wifi`, `lifegraph-linux-wifi`
  - `lifegraph-network-interface`, `lifegraph-linux-netif`
  - `lifegraph-usb`, `lifegraph-linux-usb`
  - `lifegraph-pci`, `lifegraph-linux-pci`
  - `lifegraph-nfc`, `lifegraph-linux-nfc`
  - `lifegraph-npu`, `lifegraph-linux-npu`

- **Application / experiment crates**
  - `lifegraph-agent`
  - `lifegraph-amd-xdna`

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
cargo run -q -p lifegraph-linux-pci --bin linux-pci-tool -- list
cargo run -q -p lifegraph-linux-usb --bin linux-usb-tool -- list
cargo run -q -p lifegraph-linux-netif --bin linux-netif-tool -- list
cargo run -q -p lifegraph-linux-wifi --bin linux-wifi-tool -- list
```
