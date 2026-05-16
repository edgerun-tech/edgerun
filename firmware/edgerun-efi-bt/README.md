# EdgeRun UEFI Bluetooth bring-up

C/EDK II UEFI application for visible RTL8922AE Bluetooth USB HCI bring-up.

This replaces the no-crate Rust prototype. The low-level UEFI/USB/HCI layer is C-native because UEFI is C-native and the current work is mostly ABI, descriptors, byte buffers, and HCI packet handling.

## Current milestone

The app does only diagnostic bring-up:

1. boot as `EFI/BOOT/BOOTX64.EFI`
2. locate `EFI_USB_IO_PROTOCOL` handles
3. find Realtek Bluetooth USB interface
4. discover HCI event / ACL IN / ACL OUT endpoints
5. send HCI reset
6. read HCI local version
7. read Realtek ROM version through vendor opcode `0xfc6d`
8. read BD_ADDR
9. try BLE advertising as `ER-EFI-ADMIN`
10. stay in a visible firmware loop

It does not write disks, NVRAM, boot entries, or OS files.

## Expected RTL8922AE Bluetooth USB identity

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

The code discovers endpoints from UEFI descriptors instead of hardcoding them.

## Build with EDK II

Get EDK II ready first:

```sh
git clone https://github.com/tianocore/edk2.git
cd edk2
git submodule update --init
make -C BaseTools
. ./edksetup.sh
```

Then from this repository:

```sh
firmware/edgerun-efi-bt/build-edk2.sh /path/to/edk2 /mnt/usb
```

That copies this package into the EDK II checkout, builds it as `EdgerunEfiBt.efi`, and optionally installs:

```text
/mnt/usb/EFI/BOOT/BOOTX64.EFI
```

Manual EDK II build inside the EDK II checkout:

```sh
build -a X64 -t GCC5 -b RELEASE -p EdgerunEfiBtPkg/EdgerunEfiBtPkg.dsc
```

## Firmware upload status

RTL8922AE may require firmware upload before BLE commands work reliably.

Expected firmware names from Linux firmware packaging:

```text
rtl8922au_fw.bin
rtl8922au_config.bin
```

Firmware upload is intentionally not implemented yet. The next step is a clean-room implementation of the Realtek EPATCH flow:

- parse Realtek EPATCH header/signature
- identify RTL8922A project/patch section
- read ROM version with `0xfc6d`
- select the correct patch for ROM/LMP subversion
- apply or append `rtl8922au_config.bin` exactly as Realtek expects
- fragment firmware for vendor download opcode `0xfc20`
- mark the final packet correctly
- reset controller
- re-read local version and BD_ADDR
- enable BLE advertising

Do not guess `0xfc20` packets. Bad firmware chunks can wedge the controller until power-cycle.

## Why C here

This layer is better in C than Rust because it is almost entirely:

- UEFI C ABI
- EDK II protocol calls
- USB descriptors
- fixed HCI packets
- vendor firmware byte parsing

Rust still makes sense later for policy, authentication, command framing, and signed event logs after the transport is stable.
