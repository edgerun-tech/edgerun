# Crate Layout

Core workspace crates live under `crates/`. The root `Cargo.toml` currently
registers 109 workspace members, matching the 109 crate directories under
`crates/`.

## Bare Metal

- `edgerun-unikernel`: bootable x86_64 kernel binary.
- `edgerun-bare-rt`: no_std async/runtime, timers, channels, sync primitives.
- `edgerun-platform`: CPU, interrupt, timer, memory, and platform support.
- `edgerun-ipxe`, `edgerun-tftp`, `edgerun-dhcp`: boot and network bootstrap.
- `edgerun-virtio`, `edgerun-rtl8125`: device/NIC support.

## Protocol, Data, and Storage

- `edgerun-core`, `edgerun-proto`: protocol types and generated protobuf model.
- `edgerun-stream`, `edgerun-storage`: append-only stream and storage layers.
- `edgerun-config`: configuration/state projection.
- `edgerun-json`, `edgerun-encoding`, `edgerun-url`, `edgerun-glob`,
  `edgerun-regex`, `edgerun-hpack`, `edgerun-qpack`, `edgerun-log`,
  `edgerun-error`: low-level support crates.
- `edgerun-vfs`, `edgerun-virtual-disk`: in-memory filesystem and virtual disk.

## Network Services

- `edgerun-server`: multi-protocol server crate.
- `edgerun-net`: unified DNS/DHCP service binary.
- `edgerun-dns`, `edgerun-dhcp`, `edgerun-dhcpv6`: DNS and DHCP protocols.
- `edgerun-http`, `edgerun-tls`, `edgerun-quic`, `edgerun-acme`: HTTP/TLS/QUIC.
- `edgerun-proxy`, `edgerun-oauth`: proxy and OAuth support.
- `edgerun-email`, `edgerun-email-auth`, `edgerun-mail-web`: email server,
  authentication, and webmail.

## Mesh, Node, and Scheduling

- `edgerun-node`: node daemon.
- `edgerun-mesh`, `edgerun-mesh-link`, `edgerun-mesh-session`,
  `edgerun-mesh-capability`, `edgerun-mesh-daemon`: mesh networking.
- `edgerun-remote-capability`: remote capability transport and adapters.
- `edgerun-scheduler`, `edgerun-marketplace-cli`: compute scheduling and market
  tooling.

## Security and Identity

- `edgerun-crypto`: workspace crypto boundary.
- `edgerun-capabilities`, `edgerun-capability-policy`: capability model and
  policy enforcement.
- `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey`,
  `edgerun-android-keystore`: signing backends.
- `edgerun-secret-service`: secret storage service.

## Hardware Capability Traits

- `edgerun-input`, `edgerun-microphone`, `edgerun-speaker`, `edgerun-display`,
  `edgerun-camera-biometrics`, `edgerun-fingerprint`, `edgerun-biometrics`.
- `edgerun-bluetooth`, `edgerun-wifi`, `edgerun-network-interface`,
  `edgerun-usb`, `edgerun-pci`, `edgerun-nfc`, `edgerun-npu`,
  `edgerun-power`, `edgerun-cec`, `edgerun-gpu`.

## Linux and Device Backends

- `edgerun-linux-sysfs`, `edgerun-linux-pci`, `edgerun-linux-usb`,
  `edgerun-linux-netif`, `edgerun-linux-wifi`, `edgerun-linux-nfc`,
  `edgerun-linux-npu`, `edgerun-linux-gpu`, `edgerun-linux-cec`,
  `edgerun-linux-power`.
- `edgerun-alsa-microphone`, `edgerun-alsa-speaker`, `edgerun-evdev-input`,
  `edgerun-v4l2-camera`, `edgerun-goodix-fingerprint`,
  `edgerun-mgmt-bluetooth`, `edgerun-bluetooth-gatt`, `edgerun-drm-display`,
  `edgerun-amd-xdna`, `edgerun-quectel-ec200a`.

## Applications and Integrations

- `edgerun-oci`: OCI runtime and registry client.
- `edgerun-tcl-ac`, `edgerun-tcl-ac-cli`, `edgerun-tuya`, `edgerun-matter`.
- `edgerun-solana`.
- `edgerun-edit`, `edgerun-bench`, `edgerun-e2e`,
  `edgerun-e2e-capability`.

## Notes

The workspace favors local primitives (`edgerun-bare-rt`, `edgerun-json`,
`edgerun-clap`, `edgerun-log`) over large external runtime stacks. Many crates
are `no_std` or `alloc`-first even when they also expose hosted Linux binaries.
