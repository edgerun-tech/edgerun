# Crate Organization Map

Status: implemented workspace inventory and suggested organization targets.

The workspace currently resolves 109 crate manifests. The root manifest now
lists each crate directory exactly once. This document does not move crates; it
records the current logical ownership groups so future moves or package
metadata cleanup can happen deliberately.

## Protocol Core

Implemented protocol, identity, stream, object, storage, and validation crates:

- `edgerun-core`
- `edgerun-proto`
- `edgerun-stream`
- `edgerun-storage`
- `edgerun-edgefs`
- `edgerun-crypto`
- `edgerun-hardware-signing`
- `edgerun-capabilities`
- `edgerun-capability-policy`
- `edgerun-remote-capability`
- `edgerun-config`

## Runtime And Bare Targets

Implemented runtime/platform crates and bare-target support. Some board support
is bare-target or early bring-up code rather than a complete hardware runtime.

- `edgerun-rt`
- `edgerun-platform`
- `edgerun-unikernel`
- `edgerun-virtio`
- `edgerun-rtl8125`
- `edgerun-ipxe`
- `edgerun-tftp`

## Mesh And Node

Implemented node orchestration, identity-routed networking, mesh sessions, and
mesh capability wiring:

- `edgerun-node`
- `edgerun-mesh`
- `edgerun-mesh-link`
- `edgerun-mesh-session`
- `edgerun-mesh-capability`
- `edgerun-mesh-daemon`
- `edgerun-network-interface`

## Service Protocols

Implemented or partially implemented service protocol crates:

- `edgerun-http`
- `edgerun-hpack`
- `edgerun-qpack`
- `edgerun-quic`
- `edgerun-tls`
- `edgerun-dns`
- `edgerun-dhcp`
- `edgerun-dhcpv6`
- `edgerun-email`
- `edgerun-email-auth`
- `edgerun-server`
- `edgerun-proxy`
- `edgerun-acme`
- `edgerun-oauth`
- `edgerun-oci`
- `edgerun-vfs`
- `edgerun-virtual-disk`

## Hardware Abstractions

Implemented abstract device/interface crates:

- `edgerun-android-hardware`
- `edgerun-biometrics`
- `edgerun-bluetooth`
- `edgerun-bluetooth-gatt`
- `edgerun-camera-biometrics`
- `edgerun-cec`
- `edgerun-display`
- `edgerun-drm-display`
- `edgerun-fingerprint`
- `edgerun-gpu`
- `edgerun-input`
- `edgerun-microphone`
- `edgerun-nfc`
- `edgerun-npu`
- `edgerun-pci`
- `edgerun-power`
- `edgerun-speaker`
- `edgerun-usb`
- `edgerun-wifi`

## Host Adapters

Host-only or host-adapter crates that bind abstract interfaces to Linux,
Android, ALSA, V4L2, Goodix, TPM, YubiKey, and related platform APIs:

- `edgerun-alsa-microphone`
- `edgerun-alsa-speaker`
- `edgerun-amd-xdna`
- `edgerun-android-keystore`
- `edgerun-evdev-input`
- `edgerun-goodix-fingerprint`
- `edgerun-inotify`
- `edgerun-linux-cec`
- `edgerun-linux-gpu`
- `edgerun-linux-netif`
- `edgerun-linux-nfc`
- `edgerun-linux-npu`
- `edgerun-linux-pci`
- `edgerun-linux-power`
- `edgerun-linux-sysfs`
- `edgerun-linux-usb`
- `edgerun-linux-wifi`
- `edgerun-quectel-ec200a`
- `edgerun-tpm`
- `edgerun-v4l2-camera`
- `edgerun-yubikey`

## Application Domains

Implemented application/domain crates:

- `edgerun-audio-calibration`
- `edgerun-audio-liveliness`
- `edgerun-e2e`
- `edgerun-e2e-capability`
- `edgerun-face-detection`
- `edgerun-machine-report`
- `edgerun-matter`
- `edgerun-mgmt-bluetooth`
- `edgerun-scheduler`
- `edgerun-secret-service`
- `edgerun-solana`
- `edgerun-tcl-ac`
- `edgerun-tuya`

## Tooling And Utility Crates

Host tools, proc macros, and small support libraries:

- `edgerun-bench`
- `edgerun-clap`
- `edgerun-clap-derive`
- `edgerun-edit`
- `edgerun-encoding`
- `edgerun-error`
- `edgerun-glob`
- `edgerun-json`
- `edgerun-log`
- `edgerun-mail-web`
- `edgerun-marketplace-cli`
- `edgerun-regex`
- `edgerun-tcl-ac-cli`
- `edgerun-url`

## Cleanup Targets

- Add or tighten `package.description` on crates that still lack one.
- Keep `license.workspace = true`, `publish.workspace = true`, and
  `[lints] workspace = true` consistent for publishable crates.
- Treat proc-macro crates such as `edgerun-clap-derive` and `edgerun-error` as
  host-side tooling even when they generate no_std-compatible code.
- If physical directory moves are ever done, move by group and update path
  dependencies in the same commit. Do not mix that with behavioral changes.
