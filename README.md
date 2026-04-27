# Edgerun Core

Edgerun Core is a Rust workspace for edge infrastructure, bare-metal runtime work,
mesh services, hardware capability adapters, and protocol/data-plane crates. The
workspace currently has 109 registered members in the root `Cargo.toml` and 109
crate directories under `crates/`.

The codebase is in active development. Many crates are intentionally `no_std` or
`alloc`-first so they can run in hosted Linux tools, service daemons, and the
bare-metal unikernel target.

## Main Areas

| Area | Crates |
|---|---|
| Bare metal | `edgerun-unikernel`, `edgerun-bare-rt`, `edgerun-platform`, `edgerun-ipxe`, `edgerun-tftp`, `edgerun-virtio`, `edgerun-rtl8125` |
| Protocol and storage | `edgerun-core`, `edgerun-proto`, `edgerun-stream`, `edgerun-storage`, `edgerun-config`, `edgerun-json`, `edgerun-encoding`, `edgerun-log` |
| Network services | `edgerun-server`, `edgerun-net`, `edgerun-dns`, `edgerun-dhcp`, `edgerun-dhcpv6`, `edgerun-http`, `edgerun-tls`, `edgerun-quic`, `edgerun-acme`, `edgerun-proxy`, `edgerun-oauth` |
| Mesh and scheduling | `edgerun-mesh`, `edgerun-mesh-link`, `edgerun-mesh-session`, `edgerun-mesh-capability`, `edgerun-mesh-daemon`, `edgerun-node`, `edgerun-scheduler`, `edgerun-remote-capability` |
| OCI and filesystems | `edgerun-oci`, `edgerun-virtual-disk`, `edgerun-vfs` |
| Security and identity | `edgerun-crypto`, `edgerun-capabilities`, `edgerun-capability-policy`, `edgerun-hardware-signing`, `edgerun-tpm`, `edgerun-yubikey`, `edgerun-android-keystore`, `edgerun-secret-service`, `edgerun-email-auth` |
| Hardware adapters | `edgerun-linux-*`, `edgerun-alsa-*`, `edgerun-evdev-input`, `edgerun-v4l2-camera`, `edgerun-goodix-fingerprint`, `edgerun-mgmt-bluetooth`, `edgerun-amd-xdna`, `edgerun-quectel-ec200a` |
| Capability traits | `edgerun-input`, `edgerun-microphone`, `edgerun-speaker`, `edgerun-display`, `edgerun-network-interface`, `edgerun-usb`, `edgerun-pci`, `edgerun-nfc`, `edgerun-npu`, `edgerun-power`, `edgerun-wifi`, `edgerun-bluetooth`, `edgerun-biometrics`, `edgerun-fingerprint`, `edgerun-camera-biometrics` |
| Applications and tools | `edgerun-email`, `edgerun-mail-web`, `edgerun-tcl-ac`, `edgerun-tcl-ac-cli`, `edgerun-tuya`, `edgerun-matter`, `edgerun-solana`, `edgerun-marketplace-cli`, `edgerun-edit`, `edgerun-bench` |

See [crates/README.md](crates/README.md) for the crate inventory and
[docs/INDEX.md](docs/INDEX.md) for design and component documentation.

## Build

The workspace uses Rust 2021 and a minimum Rust version of 1.81. Start with a
normal hosted build:

```bash
cargo build --workspace
```

The bare-metal unikernel requires nightly and `build-std`:

```bash
cargo +nightly build --release -p edgerun-unikernel \
  --target x86_64-unknown-none \
  -Zbuild-std=core,alloc
```

To run the QEMU wrapper:

```bash
scripts/qemu-unikernel.sh
```

## Useful Checks

```bash
# Fast compile check for all workspace members
cargo check --workspace

# Run the default test set
cargo test --workspace

# Build selected no_std/bare crates for the freestanding target
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

Some workspace crates target Linux hardware APIs and require local devices,
permissions, or kernel support for their integration tests.

## Unikernel Path

The current bare-metal path is centered on `edgerun-unikernel`:

1. The boot wrapper enters the kernel image and sets up stack/BSS.
2. `edgerun-platform` provides low-level CPU, interrupt, timer, and memory
   primitives.
3. `edgerun-bare-rt` provides async and synchronization primitives without
   depending on Tokio.
4. Device/network support is split across `edgerun-virtio`, `edgerun-rtl8125`,
   `edgerun-ipxe`, `edgerun-dhcp`, and `edgerun-tftp`.

The helper scripts under `scripts/qemu-unikernel*.sh` are the canonical local
entry points for QEMU-based boot testing.

## Protocol Specs

Protocol definitions live under `proto/edgerun/v0`. The tree currently contains
47 files including the README, covering core identity/access/streaming,
capabilities, network metadata, CSS/HTML/DOM/ECMAScript-derived data models, and
web platform support types.

## Documentation Map

- [Documentation index](docs/INDEX.md)
- [Crate layout](crates/README.md)
- [HTTP notes](docs/http/README.md)
- [TLS notes](docs/tls/README.md)
- [OCI runtime notes](docs/oci-runtime/README.md)
- [Virtual disk notes](docs/virtual-disk/README.md)
- [Remote capability notes](docs/remote-capability/README.md)
- [TPM notes](docs/security/tpm.md)

## Status Notes

- `edgerun-bare-rt` is the active runtime dependency across many crates; older
  references to a separate `edgerun-rt` crate are historical.
- Most hardware abstraction crates define traits, types, or Linux-specific
  adapters. Their completeness varies by device family.
- Browser-engine RFCs and copied web specs are still present in `docs/`, but the
  root workspace does not currently include the older generated browser crates
  that prior README revisions described.
