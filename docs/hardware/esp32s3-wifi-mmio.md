# ESP32-S3 Wi-Fi MMIO Bring-Up Notes

Status: bare-target/stubbed hardware bring-up. The code can initialize clocks,
MAC/RX descriptors, selected PHY/MMIO registers, call a few ROM PHY slots, arm a
raw 802.11 RX path, and feed received frames into the Edgerun-owned open-AP
state machine without vendor Wi-Fi/PHY static libraries. It does not yet have a
verified raw 802.11 TX descriptor/doorbell path, so it still does not provide a
usable network interface or a TV-visible AP.

## Stable Baseline

Build target:

```bash
PATH="/home/ken/.espressif/tools/xtensa-esp-elf/esp-15.2.0_20251204/xtensa-esp-elf/bin:$PATH" \
cargo +esp build --release -p edgerun-unikernel \
  --features html-ui,esp32s3-wifi-mmio \
  --target xtensa-esp32s3-none-elf -Zbuild-std=core,alloc
```

In this checkout the root Cargo config points `xtensa-esp32s3-none-elf` at the
ESP GCC linker under `devices/edgerun-tcl-usb-ap-bridge/.embuild`, so the
shorter build command also works:

```bash
cargo +esp build --release -p edgerun-unikernel \
  --features esp32s3-wifi-mmio \
  --target xtensa-esp32s3-none-elf -Zbuild-std=core,alloc
```

The generic helper script builds `esp32s3-wifi-mmio,esp32s3-headless` by
default for USB-serial Wi-Fi probing without touching the display path:

```bash
scripts/esp32s3-unikernel.sh flash
```

Stable control commands:

- `status`: board/display/touch status.
- `wifiinit`: implemented; runs the current known-good MMIO init sequence.
- `wifi`: implemented; starts the MMIO-backed open AP state machine and arms
  RX on channel 6. TX is currently stubbed at the MMIO backend boundary.
- `wifistats` / `wifi stats`: implemented; reports raw/AP bridge counters and
  the last MMIO Wi-Fi status code.
- `wififuns`: implemented; dumps ROM PHY function table slots via
  `phy_get_romfuncs`.
- `wifirx`: implemented; dumps compact RX descriptor/MMIO scratch state.
- `wifiphyrx` / `wifi51`: implemented; calls ROM function table slot `0x088`
  with argument `1`.
- `wifipbusdbg` / `wifi52`: experimental; calls the vendor-observed PBUS helper
  sequence through ROM slot `0x1a8`.
- `wifiromrx` / `wifi53`: experimental; installs the local WDEV-shaped RX
  control block into selected ROM RAM globals and points MAC RX base at that
  control block.
- `wifidmaromrx` / `wifi55`: experimental; installs the same ROM RAM globals
  but leaves MAC RX base pointing at the descriptor ring.
- `wifiromphyrx` / `wifi54` and `wifidmaromphyrx` / `wifi56`: experimental;
  run the corresponding ROM-global install then call ROM PHY slot `0x088`.

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
By default the script speaks the headless raw newline control protocol; pass
`--framed` for the older serial-mux control channel.

Tokens are separated by spaces, commas, or semicolons.

- Numeric tokens send `wifiN`.
- `init` sends `wifiinit`.
- `rx` sends `wifirx`.
- `rx6` expands to the current channel-6 RX probe sequence.
- `rxrom` tests the WDEV-control-block RX-base variant.
- `rxdmarom` tests the descriptor-ring RX-base plus ROM-global variant.
- `rxdmaromfilter` adds the RF-test MAC/filter and 2440 MHz optimization
  setup before the descriptor-ring RX-base ROM-global start.
- `rxdmaromclone` extends `rxdmaromfilter` with the current no-blob PHY-param
  scratch setup, the Rust-side channel-register clone, guarded TX-gain probe,
  AGC restore, and RF-state restore.
- `rfch-save`, `rfch-pre`, and `rfch-mode` are confirmed to return cleanly when
  run after the current descriptor-ring RX setup.
- `rfch-gain-pre` calls ROM slot `g_phyFuns + 0x24c` as `(1)` and
  `rfch-gain-ch` calls `g_phyFuns + 0x264` as `(6, 0)`. Both are currently
  hazardous when called directly: each stalls the board. The vendor path calls
  `wr_rx_gain_mem` first, using `phy_param` bytes around `0xf1` and `0x1f6`, so
  these slots probably require a generated RX gain table/context.
- `rfch-post` and `rfch-restore` are confirmed to return cleanly when run
  without the gain hooks.
- `rfchan6` runs the combined `chip_v7_set_chan` slot-path approximation. This
  is state-sensitive and currently hazardous: it returned after the normal RX
  setup plus `phyparam`, but `phyparam rfchan6` from a clean boot stalled the
  board. It must stay out of default presets.
- `phyparam` installs a local no-blob scratch buffer via `rom_phy_param_addr`.
  The scratch buffer expands the public 128-byte ESP32-S3 PHY init data into
  the runtime offsets reproduced from `register_chipv7_phy_init_param`, then
  fills the separate channel runtime fields at `0x1f2..0x1f6`. Copying the
  public 128-byte init table raw into this runtime block was tried and made the
  board stop answering serial commands on the next clean boot, so raw-copy is
  explicitly not the stable path.
- `rfsub06c`, `rfsub054`, `rfsub0c4`, and `rfsub080` split the four subcalls
  made by ROM `rom_set_chan_reg`. All four return when `rfsub0c4` receives
  scratch-buffer pointers for the RX compensation tables. `rfch-reg0` still
  stalls inside the ROM aggregate wrapper, so use `rfch-clone` instead.
- `rfch-clone` is a Rust-side no-blob clone of the safe `rom_set_chan_reg`
  subcall sequence. It returns cleanly, as do `txgain0`, `gainwrite0`, and
  `gainflat` after `phyparam`.

Example:

```bash
scripts/wifi-mmio-seq.py /dev/ttyACM0 rx6
scripts/wifi-mmio-seq.py /dev/ttyACM0 rxdmarom
scripts/wifi-mmio-seq.py /dev/ttyACM0 rxdmaromfilter
scripts/wifi-mmio-seq.py /dev/ttyACM0 rxdmaromclone
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
- Implemented in code: `wifi57` through `wifi63` split the suspected
  `chip_v7_set_chan` ROM path into individual probes. `wifi64` keeps the
  combined path as an explicit hazardous test.
- Current blocker: `g_phyFuns + 0x24c` maps to ROM `rom_set_chan_reg`, and the
  ROM aggregate wrapper still stalls even though its four subcalls return when
  invoked from Rust with explicit scratch pointers. `rfch-clone` bypasses that
  wrapper and is the stable channel-register path for current tests.
- Current blocker: `rxdmaromclone` runs to completion, but the RX descriptor
  payload sentinel remains `deadbeef`. The MAC sees the descriptor ring and ROM
  globals, but no received frame has landed in the buffer yet.
- Implemented in code: `wifi53`/`wifi54` and `wifi55`/`wifi56` can install
  descriptor/control pointers into ROM RAM globals. The descriptor-base variant
  leaves `base` on the descriptor ring and sets `romptr[2]`, `romptr[4]`, and
  `romptr[7]` to local RX structures.
- Bare-target/stubbed: `wifiphyrx` and `wifipbusdbg` execute without panicking,
  but no DMA payload lands in the RX buffer.
- Generated/type-catalog material: `librftest.a` from the local IDF checkout
  contains `esp_rx_func`, `do_rx_poll`, `get_rx_buffer`,
  `esp_phy_get_rx_result`, `WifiRxStart`, `rx_per_init`, `rx_pbus_set`, and
  `rftest_set_chan`. These are being used as reverse-engineering references,
  not linked into the no-blob build.
- Currently blocked: several ROM Wi-Fi globals (`sta_rxcb`, `g_ic_ptr`,
  `pp_wdev_funcs`, `lmacConfMib`, and related slots) still remain zero after
  the current no-blob init sequence. The local probes only seed the RX
  descriptor/WDEV pointer globals.
- Currently blocked: the full RX enable path still lacks required PHY/MAC state;
  `reload` can latch and `noise` can move, but `irq`, RX end state, and RX
  buffer contents remain zero/unchanged after the tested sequences above.
- Currently blocked: on April 29, 2026, a flashed headless image on the
  JC3248W535 reached `ready headless`, accepted `wifi`, and completed the
  `rxdmaromclone` sequence, but repeated `wifistats` still reported
  `raw_rx=0`, `raw_tx=1`, and no RX IRQ/end state.
- Currently blocked: the RF-test wrapper calls `rftest_set_chan`, which routes
  through `chip_v7_set_chan` and additional ROM PHY table entries for RX gain
  and channel-misc programming. The current direct channel start is not yet
  equivalent to that path.
- Avoid for now: direct `rom_noise_check_loop(1, 1)` caused a LoadProhibited
  panic when called without the full vendor PHY state.
- Avoid for now: adding a direct ROM `ic_mac_init` probe at `0x400052e0`
  changed the Xtensa image enough to hang during display initialization before
  the control loop. Revisit only after the boot/layout sensitivity is fixed.
