# Crate Layout

The crate tree is organized around the v0 protocol rather than around one
application. The root protocol is an identity-routed, append-only fabric:
single-writer streams, immutable objects, signed commands, explicit
capabilities/delegation, query access, and local trust policy.

## Workspace State

- `crates/` currently contains 110 first-level directories; all of them have
  `Cargo.toml` manifests.
- `cargo metadata --no-deps --format-version 1` succeeds and reports 110
  workspace packages/members.
- The root `Cargo.toml` textual `members` array has 110 unique entries.
- See [docs/project-state.md](../docs/project-state.md) for a code-grounded
  assessment of implemented, partial, generated, host-only, and bare-target
  surfaces.

## Protocol Fabric

- `edgerun-proto`: generated protobuf bindings for `proto/edgerun/v0`.
- `edgerun-core`: canonical protocol record wrapper, prost-based
  canonicalization, domain-separated hashes/signatures, validators, conformance
  vector loading.
- `edgerun-stream`: single-writer signed event streams.
- `edgerun-storage`: event log, encrypted blobs, file/block/memory stores,
  indexes, snapshots, replay cache, rebuild/integrity logic.
- `edgerun-node`: node-level command/query/store task orchestration.

## Trust, Identity, and Capabilities

- `edgerun-capabilities`: protobuf capability descriptors, selectors, requests,
  grants, invocations, results, revocations, and basic validators.
- `edgerun-capability-policy`: grant lifecycle and policy decisions.
- `edgerun-remote-capability`: signed remote capability messages, sessions,
  transports, and adapters.
- `edgerun-hardware-signing`: universal P-256 `NodeID`/`MeshSigner` model and
  TPM/YubiKey/Android adapter glue.
- `edgerun-tpm`, `edgerun-yubikey`, `edgerun-android-keystore`: concrete secure
  signing backends.
- `edgerun-secret-service`, `edgerun-email-auth`, `edgerun-crypto`: supporting
  security, secrets, authentication, and crypto boundary crates.

## Mesh and Node Transport

- `edgerun-mesh`: signed identity-addressed mesh frames and routing.
- `edgerun-mesh-link`: raw Ethernet, UDP multicast/broadcast, and IP tunnel
  link paths.
- `edgerun-mesh-session`: ECDH handshake, encrypted sessions, replay/rekey
  policy.
- `edgerun-mesh-daemon`: polling daemon that ties links, router, sessions, and
  capability dispatch together.
- `edgerun-mesh-capability`: carries capability runtime envelopes over mesh.

## Services and Protocol Stacks

- `edgerun-http`: HTTP/1.1, HTTP/2, HTTP/3 shared API with HPACK/QPACK support.
- `edgerun-tls`: async TLS 1.3 building block for HTTP/service stacks.
- `edgerun-quic`: QUIC transport structures, packet protection, handshakes, and
  transport state.
- `edgerun-dns`, `edgerun-dhcp`, `edgerun-dhcpv6`: DNS, DHCP, DHCPv6, and
  PXE/TFTP integration.
- `edgerun-server`: feature-gated multi-protocol server builder.
- `edgerun-email`: SMTP/IMAP/LMTP server/client pieces.
- `edgerun-proxy`: HTTP CONNECT/forward proxy.
- `edgerun-oci`: no_std OCI data model/parser plus std-gated runtime, registry,
  namespace, cgroup, seccomp, rootfs, and CLI modules.

## Bare Runtime and Boot

- `edgerun-rt`: no_std async runtime, executor, timers, sleep/timeout,
  channels, sync primitives, async I/O traits, and TCP/UDP/IP primitives.
- `edgerun-platform`: low-level platform primitives.
- `edgerun-unikernel`: freestanding bootable binary.
- `edgerun-ipxe`, `edgerun-tftp`, `edgerun-virtio`, `edgerun-rtl8125`: boot,
  TFTP, VirtIO, and NIC support.

## Hardware Capability Traits

Trait/type crates:

- `edgerun-input`, `edgerun-microphone`, `edgerun-speaker`, `edgerun-display`,
  `edgerun-camera-biometrics`, `edgerun-fingerprint`, `edgerun-biometrics`,
  `edgerun-bluetooth`, `edgerun-wifi`, `edgerun-network-interface`,
  `edgerun-usb`, `edgerun-pci`, `edgerun-nfc`, `edgerun-npu`,
  `edgerun-power`, `edgerun-cec`, `edgerun-gpu`.

Linux/device adapters:

- `edgerun-linux-sysfs`, `edgerun-linux-pci`, `edgerun-linux-usb`,
  `edgerun-linux-netif`, `edgerun-linux-wifi`, `edgerun-linux-nfc`,
  `edgerun-linux-npu`, `edgerun-linux-gpu`, `edgerun-linux-cec`,
  `edgerun-linux-power`.
- `edgerun-alsa-microphone`, `edgerun-alsa-speaker`, `edgerun-evdev-input`,
  `edgerun-v4l2-camera`, `edgerun-goodix-fingerprint`,
  `edgerun-mgmt-bluetooth`, `edgerun-bluetooth-gatt`, `edgerun-drm-display`,
  `edgerun-amd-xdna`, `edgerun-quectel-ec200a`.

## Local Primitives and Tools

- `edgerun-json`: zero-dependency JSON with owned, borrowed, tape, schema,
  YAML/TOML feature paths.
- `edgerun-encoding`: base encodings, varints, frames, IP/net helpers,
  protobuf helpers, RFC date helpers, TLV, percent and quoted-printable.
- `edgerun-hpack`, `edgerun-qpack`: HTTP header compression.
- `edgerun-clap`, `edgerun-clap-derive`: local CLI parser and derives.
- `edgerun-log`: minimal logging.
- `edgerun-vfs`: RAM-backed VFS with git-aware write-back.
- `edgerun-virtual-disk`: disk image, NBD, and block protocol support.
- `edgerun-edit`, `edgerun-bench`, `edgerun-e2e`,
  `edgerun-e2e-capability`: tooling and tests.

## Application/Integration Crates

- `edgerun-tcl-ac`, `edgerun-tcl-ac-cli`, `edgerun-tuya`, `edgerun-matter`.
- `edgerun-solana`, `edgerun-scheduler`, `edgerun-marketplace-cli`.
- `edgerun-mail-web`.

## EdgeFS

- `edgerun-edgefs`: no_std encrypted, append-first block filesystem built over
  `edgerun-storage::BlockStorage`, with encrypted records and rebuildable
  indexes.
