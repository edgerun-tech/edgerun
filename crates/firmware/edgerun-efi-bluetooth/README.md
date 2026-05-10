# edgerun-efi-bluetooth

No-crate UEFI Bluetooth bring-up crate for the Realtek RTL8922AE Bluetooth USB function.

The first milestone is intentionally small:

1. boot as `EFI/BOOT/BOOTX64.EFI`
2. enumerate `EFI_USB_IO_PROTOCOL` handles
3. find the Realtek Bluetooth USB interface
4. send HCI reset
5. read HCI local version
6. read Realtek ROM version through vendor HCI
7. read BD_ADDR
8. enable BLE advertising as `ER-EFI-ADMIN`

No external crates are used. The crate is its own nested Cargo workspace so it does not participate in normal host workspace builds.

## Build

From this directory:

```sh
rustup target add x86_64-unknown-uefi
cargo build --release
```

Copy the resulting EFI binary to a FAT32 USB stick:

```sh
mkdir -p /mnt/usb/EFI/BOOT
cp target/x86_64-unknown-uefi/release/edgerun-efi-bluetooth.efi /mnt/usb/EFI/BOOT/BOOTX64.EFI
sync
```

Boot the PC from the USB stick. The screen should print USB/HCI progress and then keep the firmware loop alive.

## Expected RTL8922AE USB function

The Bluetooth side of RTL8922AE is expected to appear as a USB Bluetooth controller:

```text
vendor  = 0x0bda
product = 0x8922
class   = 0xe0
sub     = 0x01
proto   = 0x01
```

Typical endpoints:

```text
interrupt IN = 0x81   HCI events
bulk OUT     = 0x02   ACL TX
bulk IN      = 0x82   ACL RX
```

The code does not hard-require the exact endpoint numbers. It discovers interrupt-IN, bulk-OUT, and bulk-IN from UEFI endpoint descriptors.

## Firmware files

RTL8922AE may require Realtek firmware before BLE commands succeed. The expected Linux firmware names are:

```text
rtl8922au_fw.bin
rtl8922au_config.bin
```

For later embedded-firmware testing, put them here:

```text
firmware/rtl8922au_fw.bin
firmware/rtl8922au_config.bin
```

Then build with:

```sh
cargo build --release --features embedded-rtl8922au-fw
```

The firmware upload parser is not implemented yet. The current code reports that the files are embedded, then returns an explicit firmware-upload error instead of sending guessed Realtek vendor packets.

## Next implementation step

Port only the RTL8922A branch from Linux `drivers/bluetooth/btrtl.c`:

- verify Realtek EPATCH signature
- read RTL ROM version using vendor opcode `0xfc6d`
- select the RTL8922A patch by ROM/LMP subversion
- combine or apply `rtl8922au_config.bin` exactly as Realtek expects
- send firmware chunks using vendor opcode `0xfc20`
- mark the final fragment correctly
- reset and re-read local version / BD_ADDR

Do not implement GATT yet. First make the controller reliably advertise from UEFI.

## Current control path

```text
UEFI application
  -> EFI_USB_IO_PROTOCOL
  -> USB Bluetooth class device
  -> HCI command/event transport
  -> Realtek vendor init
  -> BLE advertising
```

## Safety model

This crate should stay visible and consent-based. It prints to the local screen and does not modify disks, NVRAM, boot entries, or the installed OS.
