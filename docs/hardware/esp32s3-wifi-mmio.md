# ESP32-S3 Wi-Fi MMIO Bring-Up Notes

Status: bare-target/stubbed hardware bring-up. The code can initialize clocks,
MAC/RX descriptors, selected PHY/MMIO registers, and call a few ROM PHY slots
without vendor Wi-Fi/PHY static libraries. It does not yet receive Wi-Fi frames
or provide a network interface.

## Stable Baseline

Build target:

```bash
PATH="/home/ken/.espressif/tools/xtensa-esp-elf/esp-15.2.0_20251204/xtensa-esp-elf/bin:$PATH" \
cargo +esp build --release -p edgerun-unikernel \
  --features html-ui,esp32s3-wifi-mmio \
  --target xtensa-esp32s3-none-elf -Zbuild-std=core,alloc
```

Stable control commands:

- `status`: board/display/touch status.
- `wifiinit`: implemented; runs the current known-good MMIO init sequence.
- `wififuns`: implemented; dumps ROM PHY function table slots via
  `phy_get_romfuncs`.
- `wifirx`: implemented; dumps compact RX descriptor/MMIO scratch state.
- `wifiphyrx` / `wifi51`: implemented; calls ROM function table slot `0x088`
  with argument `1`.
- `wifipbusdbg` / `wifi52`: experimental; calls the vendor-observed PBUS helper
  sequence through ROM slot `0x1a8`.

Known stable ROM table from the JC3248W535 ESP32-S3:

```text
table=0x3fcef3d8
008=0x40038020 00c=0x40038068 05c=0x40038fc8 06c=0x40039074
078=0x400387e0 088=0x400388e0 0c8=0x40038e60 0d0=0x400391c0
0fc=0x400366c0 100=0x40036714 110=0x400368c4 148=0x40036c28
160=0x40055bb8 164=0x40055bc0 190=0x40035880 198=0x400358d8
1a8=0x40035acc 1b4=0x40035b80 1d4=0x40035e3c 200=0x40036210
204=0x40036230 208=0x4003627c 20c=0x40036d50 224=0x40037050
22c=0x400371f0 234=0x4003726c 254=0x40037600 264=0x40037894
268=0x40037958 288=0x40037d74 28c=0x40037d8c
```

## Fast Iteration

Use the host-side sequence runner to test multiple orderings without reflashing.
This intentionally lives outside the firmware because adding large board-side
test runners changed Xtensa image layout enough to hang the current display init.

Tokens are separated by spaces, commas, or semicolons.

- Numeric tokens send `wifiN`.
- `init` sends `wifiinit`.
- `rx` sends `wifirx`.
- `rx6` expands to the current channel-6 RX probe sequence.

Example:

```bash
scripts/wifi-mmio-seq.py /dev/ttyACM0 rx6
scripts/wifi-mmio-seq.py /dev/ttyACM0 init rx 52 rx 50 35 51 40 41 42 rx
scripts/wifi-mmio-seq.py /dev/ttyACM0 init rx 32 43 47 48 50 35 51 37 41 42 rx
scripts/wifi-mmio-seq.py /dev/ttyACM0 init rx 46 32 43 47 48 50 35 51 37 41 42 rx
```

This avoids reflashing for each ordering/timing assumption and keeps sequence
output grouped in one terminal run.

## Current Observations

- Implemented in code: ROM linker scripts are linked for Xtensa no-blob builds,
  while proprietary Wi-Fi/PHY/coex static libraries remain gated behind
  `esp32s3-wifi-blob`.
- Implemented in code: RX descriptor ring setup is visible to the MAC; `wifirx`
  shows descriptor addresses and the sentinel buffer.
- Implemented in code: `wififuns` dumps ROM PHY slots without vendor libraries.
- Bare-target/stubbed: `wifiphyrx` and `wifipbusdbg` execute without panicking,
  but no DMA payload lands in the RX buffer.
- Currently blocked: vendor RF-test RX entry points in the recovered `wifi.o`
  reference unresolved harness functions (`esp_rx_func`, `do_rx_poll`,
  `get_rx_buffer`, `esp_get_rx_result`). Those symbols were not found in the
  installed ESP archives either, so this is not a simple link-time gap.
- Currently blocked: ROM Wi-Fi globals (`sta_rxcb`, `g_ic_ptr`,
  `wDevCtrl_ptr`, `pp_wdev_funcs`, `pTxRx`, and related slots) remain zero
  after the current no-blob init sequence.
- Currently blocked: the full RX enable path still lacks required PHY/MAC state;
  `reload` can latch and `noise` can move, but `irq`, RX end state, and RX
  buffer contents remain zero/unchanged after the tested sequences above.
- Avoid for now: direct `rom_noise_check_loop(1, 1)` caused a LoadProhibited
  panic when called without the full vendor PHY state.
- Avoid for now: adding a direct ROM `ic_mac_init` probe at `0x400052e0`
  changed the Xtensa image enough to hang during display initialization before
  the control loop. Revisit only after the boot/layout sensitivity is fixed.
- Avoid for now: adding `wifi53`/`wifi54` probes for manually installing
  `wDevCtrl_ptr` and calling ROM `wDev_AppendRxBlocks` also changed the image
  enough to stop control-loop responses after flash. Revisit only after the
  boot/layout sensitivity is fixed.
