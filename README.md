# Lifegraph reference core

This repository is a Rust-first reference workspace for Lifegraph core protocol and machine capability experimentation.

It currently contains:
- `rust/` — the main Rust workspace
- `proto/` — Lifegraph protobuf schemas
- `docs/` — implementation and deployment notes
- `corpus/` and `spec/` — protocol conformance test vectors and specifications
- `systemd/` — service-related files

## Repository status

This repo is active, but it is not a polished end-user product yet.

What is real today:
- a multi-crate Rust workspace for Lifegraph core types, protocol bindings, remote capability plumbing, and hardware capability backends
- runnable Linux hardware inspection tools for PCI, USB, network interfaces, and Wi-Fi
- protocol/schema assets under `proto/`

What is still rough:
- top-level docs are still catching up to the current Rust workspace
- several crates are backend scaffolding rather than finished production integrations
- some hardware paths are read-only inventory today, while others expose control operations that should be used carefully

## Repository layout

- `rust/` — main workspace and crates
- `rust/crates/lifegraph-core` — core protocol/domain types and validators
- `rust/crates/lifegraph-proto` — generated protobuf bindings/build glue
- `rust/crates/lifegraph-linux-*` — Linux capability backends and CLI tools
- `proto/` — protobuf definitions
- `docs/` — notes and design docs
- `spec/` — reference/spec material
- `corpus/` — conformance test vectors

## Quick start

Basic workspace check:

```bash
cd rust
cargo check --workspace
```

Run the full test suite:

```bash
cd rust
cargo test --workspace
```

Run the currently useful read-only inventory tools:

```bash
cd rust
cargo run -q -p lifegraph-linux-pci --bin linux-pci-tool -- list
cargo run -q -p lifegraph-linux-usb --bin linux-usb-tool -- list
cargo run -q -p lifegraph-linux-netif --bin linux-netif-tool -- list
cargo run -q -p lifegraph-linux-wifi --bin linux-wifi-tool -- list
```

Useful per-interface state checks:

```bash
cd rust
cargo run -q -p lifegraph-linux-netif --bin linux-netif-tool -- state wlan0
cargo run -q -p lifegraph-linux-wifi --bin linux-wifi-tool -- state wlan0
cargo run -q -p lifegraph-linux-wifi --bin linux-wifi-tool -- scan wlan0
```

## Current CLI tools

Verified in this environment:
- `linux-pci-tool [list|inventory]`
- `linux-usb-tool [list|inventory]`
- `linux-netif-tool list | state <ifname> | up <ifname> | down <ifname>`
- `linux-wifi-tool list | state <ifname> | scan <ifname> | ap-state <ifname> | ap-start <ifname> <ssid> [freq_mhz] | ap-stop <ifname> | enable <ifname> | disable <ifname> | block <ifname>`

Be careful with commands that change interface/admin/power state; the `list`, `inventory`, `state`, and `scan` flows are the safest starting points.

## Remote capability socket demo

There is now a small remote capability demo over real sockets:

```bash
./scripts/tcp_capability_demo.sh
```

Or run the binaries directly:

```bash
cd rust
cargo run -q -p lifegraph-remote-capability --bin capability-demo-server -- tcp 127.0.0.1:47070
cargo run -q -p lifegraph-remote-capability --bin capability-demo-client -- tcp 127.0.0.1:47070
```

See also: `rust/crates/lifegraph-remote-capability/README.md`.

## Related docs

- `rust/README.md`
- `proto/README.md`
- `docs/machine_daemon.md`
- `docs/production.md`
- `spec/conformance.md`
- `spec/validator-matrix.md`
