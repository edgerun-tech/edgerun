# RFC-0012: Hardware Discovery & Linux Backends

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

Hardware discovery crates provide Linux sysfs/procfs-based device enumeration and capability reporting for GPUs, PCI devices, USB devices, network interfaces, WiFi adapters, NFC readers, NPU accelerators, power management, and CEC adapters.

---

## Architecture

```
Linux sysfs/procfs
    ↓ read files
Platform-specific discovery structs
    ↓ into_info()
Platform-agnostic types (edgerun-gpu, edgerun-pci, etc.)
    ↓ implement CapabilityProvider trait
edgerun-capabilities integration
```

---

## GPU Discovery

### edgerun-gpu

**Status:** ✅ Functional
**Tests:** None in this crate (types only)

**Purpose:** Platform-agnostic GPU abstraction. Defines:
- `GpuVendor` enum (Amd, Intel, Nvidia, Qualcomm, Apple, Arm, Matrox, Aspeed, Virtio, Microsoft, Unknown)
- `GpuInfo` struct (vendor, PCI IDs, DRM nodes, driver, PCIe link info, capability flags)
- `GpuInventory` trait — `list_gpus(&self) -> Result<Vec<GpuInfo>>`
- `infer_gpu_vendor(vendor_id) -> GpuVendor` — PCI vendor ID mapping
- `default_gpu_descriptor()` — Capability descriptor with Display + Visual + Computational modalities

**Dependencies:** Only `edgerun-capabilities`

### edgerun-linux-gpu

**Status:** ✅ Functional
**CLI:** `cargo run -p edgerun-linux-gpu --bin linux-gpu-tool -- list`
**Lines:** ~976

**Purpose:** Linux GPU discovery via `/sys/bus/pci/devices` and `/sys/class/drm`.

**Public API:**
- `LinuxGpuDevice` — pci_address, sysfs_path, vendor/device IDs, driver, DRM nodes, connectors
- `LinuxGpuBackend` — implements `CapabilityProvider` + `GpuInventory`
- `discover_gpus() -> Vec<LinuxGpuDevice>` — Main entry point
- `discover_gpus_in(pci_root, drm_root) -> Vec<LinuxGpuDevice>` — Testable variant
- `LinuxGpuDevice::into_info() -> GpuInfo` — Convert to platform-agnostic type
- `cec_adapter_lookup()`, `attach_cec_adapters_to_gpus()` — CEC resolution

**Discovery Process:**
1. Scan `/sys/bus/pci/devices` for PCI class 0x03 (Display) or 0x12 (Processing Accelerator)
2. Read vendor/device IDs, driver, boot_vga, link speed/width
3. Map DRM nodes (cardN, renderDN, connectors)
4. Parse display modes from connector sysfs
5. Resolve CEC adapters

---

## Other Hardware Discovery Crates

### Type-Only Crates (Generated from Proto)

| Crate | Purpose |
|-------|---------|
| `edgerun-pci` | PCI device types |
| `edgerun-usb` | USB device types |
| `edgerun-nfc` | NFC device types |
| `edgerun-wifi` | WiFi types |
| `edgerun-power` | Power management types |
| `edgerun-npu` | NPU types |
| `edgerun-cec` | CEC types |
| `edgerun-bluetooth` | Bluetooth types |
| `edgerun-display` | Display types |
| `edgerun-touch` | Touch event types |
| `edgerun-input` | Input device types |
| `edgerun-microphone` | Microphone types |
| `edgerun-speaker` | Speaker types |

### Linux Backend Crates

| Crate | Status | Purpose |
|-------|--------|---------|
| `edgerun-linux-pci` | ✅ Functional | Linux PCI device discovery via sysfs |
| `edgerun-linux-usb` | ⚠️ Partial | Linux USB device discovery |
| `edgerun-linux-netif` | ✅ Functional | Linux network interface discovery |
| `edgerun-linux-wifi` | ⚠️ Partial | Linux WiFi device discovery |
| `edgerun-linux-nfc` | ⚠️ Partial | Linux NFC device discovery |
| `edgerun-linux-power` | ⚠️ Partial | Linux power management |
| `edgerun-linux-npu` | ⚠️ Partial | Linux NPU device discovery |
| `edgerun-linux-cec` | ⚠️ Partial | Linux CEC (HDMI Consumer Electronics Control) |
| `edgerun-linux-sysfs` | ✅ Functional | Linux sysfs abstraction (used by all above) |

### Specialized Hardware

| Crate | Status | Purpose |
|-------|--------|---------|
| `edgerun-drm-display` | ⚠️ Partial | DRM/KMS display output |
| `edgerun-evdev-input` | ⚠️ Partial | Linux evdev input handling |
| `edgerun-amd-xdna` | ⚠️ Partial | AMD XDNA NPU support |

---

## CLI Tools

All Linux backend crates provide CLI binaries for inventory listing:

```bash
cargo run -q -p edgerun-linux-pci --bin linux-pci-tool -- list
cargo run -q -p edgerun-linux-usb --bin linux-usb-tool -- list
cargo run -q -p edgerun-linux-netif --bin linux-netif-tool -- list
cargo run -q -p edgerun-linux-wifi --bin linux-wifi-tool -- list
```

> Note: These require `all-hardware` feature flag since hardware crates are optional dependencies.

---

## edgerun-linux-sysfs (Shared Abstraction)

**Status:** ✅ Functional

**Purpose:** Common sysfs reading abstraction used by all Linux backend crates. Provides type-safe sysfs file reading with error handling.

---

## Known Issues

1. **Most Linux backends are partial** — Only `edgerun-linux-gpu`, `edgerun-linux-pci`, `edgerun-linux-netif`, and `edgerun-linux-sysfs` are functional
2. **Type crates have no behavior** — 12 generated type crates provide only proto bindings
3. **Optional dependencies** — Hardware crates excluded from default workspace to avoid compilation failures on systems without required headers
4. **No hotplug support** — Discovery is one-shot at startup, no dynamic device addition/removal
5. **No power management** — `edgerun-linux-power` is partial; no actual power state management
6. **NPU support limited** — Only AMD XDNA driver; no Intel, Qualcomm, or other NPU backends
