# edgerun Crates: Hardware Capability Support and Gaps

This document summarizes each crate in `crates/` with:
- what is implemented (surface API / traits / features)
- what is missing to reach a *full hardware capability* implementation

> **Note:** Many hardware adapter crates are optional dependencies of `edgerun-node`
> (under the `all-hardware` feature flag) and are excluded from the default workspace
> to avoid compilation failures on systems without the required hardware headers.

## 3. edgerun-alsa-microphone
- Has: backend stub, capability descriptor, trait implementationscode
- Missing: complete microphone capture pipeline, device open/read, full ALSA integration

## 4. edgerun-alsa-speaker
- Has: backend info, playback card/PCM info, backend capability provider
- Missing: streaming output implementation (ring buffers, format conversion) inside this crate; minimal driver wrapper

## 5. edgerun-amd-xdnacode
- Has: NPU discovery + capability metadata for AMD xDNA
- Missing: command execution path, workload submission and response polling

## 6. edgerun-android-keystore
- Has: provider traits, key metadata, signing helpers
- Missing: key lifecycle operations (generate/import/delete), full Android keystore handshake flows

## 7. edgerun-audio-calibration
- Has: calibration request/response types and analysis helpers
- Missing: direct audio I/O or integration with capture/playback hardware

## 8. edgerun-biometrics
- Has: biometric state, modality enums, policy helpers
- Missing: direct sensor pipeline (delegate to camera/fingerprint backends)

## 9. edgerun-bluetooth
- Has:
  - `BluetoothScanner` scan_nearby
  - `BluetoothConnectionProvider` list_connections
  - beacon, connection info models
- Missing (full BLE):
  - GATT client/server (discover/read/write/notify)
  - connect/disconnect/control over LE links
  - security (pair/bond/SMP), MTU, L2CAP channels
  - active scan/filter params and advertisement control

## 10. edgerun-camera-biometrics
- Has: biometric camera traits + validation patterns
- Missing: platform camera capture and HAL binding (in v4l2 adapter)

## 11. edgerun-capabilities
- Has: core capability provider traits, descriptor/operation model
- Missing: actual hardware runtime enforcement; abstract layer only

## 12. edgerun-core
- Has: core primitives and non-device-specific APIs (crypto/normalization)
- Missing: explicit hardware capability contract; not a platform driver crate

## 13. edgerun-display
- Has: `DisplayDevice` trait, mode/config descriptors
- Missing: concrete display pathway (render/commit) in this crate

## 14. edgerun-drm-display
- Has: DRM connector discovery, mode info, backend provider
- Missing: atomic/composable modesetting and full pipeline state commit

## 15. edgerun-evdev-input
- Has: evdev discovery/metadata and backend provider
- Missing: event loop plus device-specific input translation

## 16. edgerun-fingerprint
- Has: reader trait, enroll/verify structures, policies
- Missing: sensor implementation (Goodix adapter only; generic devices missing)

## 17. edgerun-goodix-fingerprint
- Has: Goodix protocol parsing, USB packet flows, enrollment control
- Missing: generic fingerprint interface and extensive error conditions mapping

## 18. edgerun-hardware-signing
- Has: provider composition for Android, TPM, YubiKey
- Missing: full physical transport and device discovery beyond adapters

## 19. edgerun-input
- Has: generic `InputDevice` trait, event type union, descriptors
- Missing: backends (evdev, windows, etc.) and runtime event consumer

## 20. edgerun-linux-netif
- Has: Linux interface enumeration, flags, capabilities
- Missing: network configuration (IP, routes, DHCP), advanced state updates

## 21. edgerun-linux-nfc
- Has: NFC adapter enumeration provider
- Missing: tag read/write, peer-to-peer activation, SNEP/LLCP implementation

## 22. edgerun-linux-npu
- Has: Linux NPU backend discovery and capability provider
- Missing: command queue and model execution API

## 23. edgerun-linux-pci
- Has: PCI bus scanning and device enumeration
- Missing: full driver binding, resource mapping, interrupt/setup flows

## 24. edgerun-linux-usb
- Has: USB device discovery and descriptor modeling
- Missing: transfer endpoints, bulk/iso/ctl communications

## 25. edgerun-linux-wifi
- Has: Wi-Fi interface backend + capability provider
- Missing: network connection control (WPA, SSID/auth management) in this crate

## 26. edgerun-mgmt-bluetooth
- Has: BlueZ mgmt API device/controller operations
- Missing: GATT profile and BLE streaming actions, module-level policy for connectivity

## 27. edgerun-microphone
- Has: microphone trait and capture request types
- Missing: device-specific adapters, buffering, format negotiation

## 28. edgerun-network-interface
- Has: abstract interface control trait and descriptor
- Missing: per-OS implementation (linux, windows) plus config mutation operations

## 29. edgerun-nfc
- Has: `NfcDevice` trait and target observation types
- Missing: tag read/write and card emulation on platform-specific paths

## 30. edgerun-npu
- Has: NPU device trait, request/result types, operation modes
- Missing: backend implementation (e.g., linux-npu, amd-xdna), scheduler and load management

## 31. edgerun-pci
- Has: PCI inventory trait and device model
- Missing: direct bus config access, userland driver controls

## 32. edgerun-proto
- Has: protobuf message bindings and generated types
- Missing: explicit hardware onboarding logic; this is wire-format only

## 33. edgerun-remote-capability
- Has: remote provider/transport protocol and proxy adapters for each capability
- Missing: network transport backend and production security (auth/encryption)

## 34. edgerun-speaker
- Has: speaker trait, playback request/result, capability descriptor
- Missing: platform-specific playback driver (ALSA, etc.) in this crate

## 35. edgerun-tpm
- Has: TPM command model, key operations, and Linux TPM adapter
- Missing: fully conformant TSS policy engine; hardware TPM behaviors still in lower primitives

## 36. edgerun-usb
- Has: USB inventory trait, device/interface model
- Missing: detailed transfer APIs and endpoint I/O

## 37. edgerun-v4l2-camera
- Has: V4L2 probe/capture, and camera-biometric specifics
- Missing: cross-platform camera capability (non-Linux), advanced queue controls

## 38. edgerun-wifi
- Has: Wifi scanner/controller/AP traits and result structs
- Missing: sophisticated connection flows, credential management, AP mode traffic handling

## 39. edgerun-yubikey
- Has: YubiKey reader probe, PIV/APDU signing path
- Missing: full OTP/U2F interfaces (only subset currently)

---

## Hardware capability gap categories

1. Device discovery only (no control or I/O): `linux-pci`, `linux-usb`, `linux-netif`, `linux-nfc`, `evdev-input`, `drm-display`, `amd-xdna`.
2. Control-only interface with no runtime I/O: `bluetooth`, `wifi`, `display`, `network-interface`, `npu`, `tpm`.
3. Protocol layer not exposed: `bluetooth` (GATT), `wifi` (WPA/hostapd), `nfc` (LLCP), `usb` (HCI transfers).
4. Mediation / remote wrappers: `remote-capability` adds RPC layer but no hardware backend.

---

## How to use this document

- Reference for implementers who need to add platform-specific provider crates
- Basis for capability roadmap: identify traits vs hardware bindings gaps
- Tracks “must-have” features for full stack support in each domain
