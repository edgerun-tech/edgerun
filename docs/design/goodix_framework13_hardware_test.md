# Goodix Framework 13 hardware test status

Tested on the Framework laptop running Arch Linux with the integrated Goodix reader:

- USB ID: `27c6:609c`
- Product: `Goodix Fingerprint USB Device`
- Serial: `UID10FF3D69_XXXX_MOC_B0`

## Confirmed working now

The backend can now talk to the real device at the USB transport layer.

Two host-side bugs were fixed:

1. `USBDEVFS_CLAIMINTERFACE` / `USBDEVFS_RELEASEINTERFACE` ioctl direction was wrong.
2. Config-descriptor probing parsed the 9-byte descriptor header as if it were the full payload.

After those fixes, live probing works and reports the real interface and endpoint layout:

- `transport.interface = 0`
- `transport.bulk_in = Some(131)`
- `transport.bulk_out = Some(1)`
- `transport.interrupt_in = None`
- `usb.vendor_id = 27c6`
- `usb.product_id = 609c`
- `usb.active_configuration = 0`
- `usb.num_interfaces = 1`
- `usb.manufacturer = Some("Goodix Technology Co., Ltd.")`
- `usb.product = Some("Goodix Fingerprint USB Device")`
- `usb.serial = Some("UID10FF3D69_XXXX_MOC_B0")`
- `goodix.version = unavailable`

## Current protocol-level failures

USB access is no longer the blocker. Failures are now at the Goodix protocol/runtime layer:

- `list` → connection timed out
- `finger-mode-status` → invalid Goodix packet CRC32
- `begin-template` → short Goodix packet
- `verify-live` → invalid Goodix packet CRC32

A raw debug `GET_VERSION` exchange shows:

- command write succeeds
- an ACK-like packet comes back
- no follow-up version payload arrives before timeout

## Current conclusion

The implementation is now genuinely reaching the reader, but one or more of these is still wrong:

- packet framing / chunk assembly
- ACK vs follow-up data sequencing
- command payload shape
- CRC or returned-packet interpretation

## What this changed

This test was still highly useful:

- transport permissions and interface claim issues are solved
- real hardware communication is confirmed
- remaining failures are now real protocol bugs, not host access bugs

## Best next fingerprint step

1. Capture raw packet dumps for failing commands.
2. Tighten ACK/data read sequencing around real device behavior.
3. Compare live responses against libfprint expectations.
