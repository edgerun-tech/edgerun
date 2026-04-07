# Crate layout

Core workspace crates live under `rust/crates/`.

- `edgerun-core`, `edgerun-proto`, `edgerun-json`, `edgerun-capabilities`,
  `edgerun-machine-report`, `edgerun-remote-capability` (protocol core)
- Input/media: `edgerun-input`, `edgerun-evdev-input`, `edgerun-microphone`,
  `edgerun-alsa-microphone`, `edgerun-speaker`, `edgerun-alsa-speaker`,
  `edgerun-audio-calibration`, `edgerun-camera-biometrics`, `edgerun-v4l2-camera`,
  `edgerun-display`, `edgerun-drm-display`
- Security/adapters: `edgerun-fingerprint`, `edgerun-goodix-fingerprint`,
  `edgerun-biometrics`, `edgerun-tpm`, `edgerun-yubikey`,
  `edgerun-android-keystore`, `edgerun-hardware-signing`
- Storage abstractions: `edgerun-virtual-disk`
- Connectivity backends: `edgerun-bluetooth`, `edgerun-mgmt-bluetooth`,
  `edgerun-wifi`, `edgerun-linux-wifi`, `edgerun-network-interface`,
  `edgerun-linux-netif`, `edgerun-usb`, `edgerun-linux-usb`, `edgerun-pci`,
  `edgerun-linux-pci`, `edgerun-nfc`, `edgerun-linux-nfc`, `edgerun-npu`,
  `edgerun-linux-npu`
- Application/examples: `edgerun-amd-xdna`
