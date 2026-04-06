# Crate layout

Core workspace crates live under `rust/crates/`.

- `lifegraph-core`, `lifegraph-proto`, `lifegraph-json`, `lifegraph-capabilities`,
  `lifegraph-machine-report`, `lifegraph-remote-capability` (protocol core)
- Input/media: `lifegraph-input`, `lifegraph-evdev-input`, `lifegraph-microphone`,
  `lifegraph-alsa-microphone`, `lifegraph-speaker`, `lifegraph-alsa-speaker`,
  `lifegraph-audio-calibration`, `lifegraph-camera-biometrics`, `lifegraph-v4l2-camera`,
  `lifegraph-display`, `lifegraph-drm-display`
- Security/adapters: `lifegraph-fingerprint`, `lifegraph-goodix-fingerprint`,
  `lifegraph-biometrics`, `lifegraph-tpm`, `lifegraph-yubikey`,
  `lifegraph-android-keystore`, `lifegraph-hardware-signing`
- Storage abstractions: `lifegraph-virtual-disk`
- Connectivity backends: `lifegraph-bluetooth`, `lifegraph-mgmt-bluetooth`,
  `lifegraph-wifi`, `lifegraph-linux-wifi`, `lifegraph-network-interface`,
  `lifegraph-linux-netif`, `lifegraph-usb`, `lifegraph-linux-usb`, `lifegraph-pci`,
  `lifegraph-linux-pci`, `lifegraph-nfc`, `lifegraph-linux-nfc`, `lifegraph-npu`,
  `lifegraph-linux-npu`
- Application/examples: `lifegraph-amd-xdna`
