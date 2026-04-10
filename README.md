# Rust workspace

This directory contains a multi-crate Rust workspace for the edgerun protocol surface and capability backends.

**Status:** Active development — core protocol functional, security hardening in progress.  
See [docs/capability-gap-analysis.md](docs/capability-gap-analysis.md) for detailed implementation status and known gaps.

The workspace is organized into crate groups by ownership/risk boundary:

- **Protocol core**
  - `edgerun-core`
  - `edgerun-proto`
  - `edgerun-json` (directory exists, not in workspace — optional dependency)
  - `edgerun-capabilities`
  - `edgerun-machine-report` (directory exists, not in workspace — optional dependency)
  - `edgerun-remote-capability`

- **Media & input abstractions**
  - `edgerun-input`, `edgerun-evdev-input`
  - `edgerun-microphone`, `edgerun-alsa-microphone`
  - `edgerun-speaker`, `edgerun-alsa-speaker`
  - `edgerun-audio-calibration` (directory exists, not in workspace — optional dependency)
  - `edgerun-camera-biometrics`
  - `edgerun-v4l2-camera`
  - `edgerun-display` (directory exists, not in workspace — optional dependency)
  - `edgerun-drm-display` (directory exists, not in workspace — optional dependency)

- **Security + identity adapters**
  - `edgerun-fingerprint` (directory exists, not in workspace — optional dependency)
  - `edgerun-goodix-fingerprint` (directory exists, not in workspace — optional dependency)
  - `edgerun-biometrics`
  - `edgerun-tpm`, `edgerun-yubikey`
  - `edgerun-android-keystore`, `edgerun-hardware-signing`

- **Connectivity/backends**
  - `edgerun-bluetooth`
  - `edgerun-mgmt-bluetooth` (directory exists, not in workspace — optional dependency)
  - `edgerun-wifi`
  - `edgerun-linux-wifi` (directory exists, not in workspace — optional dependency)
  - `edgerun-network-interface`, `edgerun-linux-netif`
  - `edgerun-usb` (directory exists, not in workspace — optional dependency)
  - `edgerun-linux-usb` (directory exists, not in workspace — optional dependency)
  - `edgerun-pci` (directory exists, not in workspace — optional dependency)
  - `edgerun-linux-pci` (directory exists, not in workspace — optional dependency)
  - `edgerun-nfc` (directory exists, not in workspace — optional dependency)
  - `edgerun-linux-nfc` (directory exists, not in workspace — optional dependency)
  - `edgerun-npu` (directory exists, not in workspace — optional dependency)
  - `edgerun-linux-npu` (directory exists, not in workspace — optional dependency)

- **Mesh networking**
  - `edgerun-mesh`, `edgerun-mesh-link`, `edgerun-mesh-router`
  - `edgerun-mesh-session` (directory exists, not in workspace — optional dependency)
  - `edgerun-mesh-capability` (directory exists, not in workspace — optional dependency)
  - `edgerun-mesh-daemon` (directory exists, not in workspace — optional dependency)

- **Runtime & infrastructure**
  - `edgerun-node` (produces the `edgerund` binary)
  - `edgerun-storage`, `edgerun-stream`, `edgerun-rt`
  - `edgerun-oci-runtime`, `edgerun-oci-registry`
  - `edgerun-capability-policy`
  - `edgerun-linux-sysfs`

- **Application / experiment crates**
  - `edgerun-amd-xdna` (directory exists, not in workspace — optional dependency)

> **Note:** Crates marked "(directory exists, not in workspace — optional dependency)" are
> available as optional dependencies of `edgerun-node` under the `all-hardware` feature flag.
> They are excluded from the default workspace to avoid compilation failures on systems
> without the required hardware headers. Build them with:
> `cargo build -p edgerun-node --features all-hardware`

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

> **Note:** The inventory CLI examples above require the `all-hardware` feature since the
> referenced crates are optional dependencies. Build with:
> `cargo run -q -p edgerun-node --features all-hardware -- <command>`
