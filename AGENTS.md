# Edgerun Core Agent Notes

This repository is a large Rust workspace for edge services, bare-metal runtime
work, mesh networking, hardware capability adapters, protocol crates, and a
unikernel target.

## Working Directory

```bash
cd /home/ken/edgerun_core
```

## Default Hosted Checks

Use these first when validating normal workspace changes:

```bash
cargo check --workspace
cargo test --workspace
```

Many crates touch Linux hardware APIs or service daemons. If a full workspace
test fails because the host lacks devices, permissions, or kernel features,
rerun the relevant crate-specific test and document the limitation.

## Unikernel Build

The unikernel is built for the freestanding x86_64 target and requires nightly
Rust with `build-std`:

```bash
cargo +nightly build --release -p edgerun-unikernel \
  --target x86_64-unknown-none \
  -Zbuild-std=core,alloc
```

To produce a flat binary:

```bash
/usr/bin/objcopy -O binary \
  target/x86_64-unknown-none/release/edgerun-unikernel \
  /tmp/edgerun.bin
```

Local QEMU boot helpers:

```bash
scripts/qemu-unikernel.sh
scripts/qemu-unikernel-net-pump.sh
scripts/qemu-unikernel-swtpm.sh
```

## Bare-Metal Architecture

- `edgerun-unikernel`: bootable kernel binary and linker/boot glue.
- `edgerun-platform`: CPU, timer, IRQ, memory, and low-level platform support.
- `edgerun-bare-rt`: no_std async/runtime primitives, synchronization, timers,
  and hosted compatibility shims.
- `edgerun-ipxe`, `edgerun-tftp`, `edgerun-dhcp`: boot/network support.
- `edgerun-virtio`, `edgerun-rtl8125`: NIC/device support.

Older docs may mention `edgerun-rt`; the current workspace uses
`edgerun-bare-rt` broadly instead.

## no_std / Bare Target Smoke Test

This list tracks the current bare-oriented crates rather than every crate that
contains `#![no_std]`:

```bash
for crate in \
  edgerun-glob edgerun-regex edgerun-url edgerun-log edgerun-json \
  edgerun-encoding edgerun-hpack edgerun-qpack edgerun-bare-rt \
  edgerun-platform edgerun-virtio edgerun-ipxe edgerun-tftp \
  edgerun-rtl8125 edgerun-http edgerun-tls edgerun-unikernel
do
  cargo +nightly build -p "$crate" --release \
    --target x86_64-unknown-none \
    -Zbuild-std=core,alloc
done
```

## Current Workspace Shape

- Root `Cargo.toml` has 109 workspace members.
- `crates/` contains 109 crate directories.
- Most library crates are `no_std` or `alloc`-first.
- `proto/edgerun/v0` contains the protocol definitions used by `edgerun-proto`.

Major crate families:

- Services: `edgerun-server`, `edgerun-net`, `edgerun-dns`, `edgerun-dhcp`,
  `edgerun-http`, `edgerun-email`, `edgerun-proxy`, `edgerun-oci`.
- Mesh/node: `edgerun-node`, `edgerun-mesh*`, `edgerun-scheduler`,
  `edgerun-remote-capability`.
- Security: `edgerun-crypto`, `edgerun-tls`, `edgerun-tpm`,
  `edgerun-yubikey`, `edgerun-hardware-signing`, `edgerun-secret-service`.
- Hardware: `edgerun-linux-*`, `edgerun-alsa-*`, `edgerun-evdev-input`,
  `edgerun-v4l2-camera`, `edgerun-goodix-fingerprint`,
  `edgerun-mgmt-bluetooth`.
- Traits/capabilities: `edgerun-input`, `edgerun-microphone`,
  `edgerun-speaker`, `edgerun-display`, `edgerun-network-interface`,
  `edgerun-usb`, `edgerun-pci`, `edgerun-nfc`, `edgerun-npu`,
  `edgerun-power`, `edgerun-wifi`, `edgerun-bluetooth`.

## Documentation Caveat

Some docs under `docs/` are historical design material or copied upstream web
specification references. Prefer the root `Cargo.toml`, crate source, and
crate-local READMEs when deciding what is currently implemented.
