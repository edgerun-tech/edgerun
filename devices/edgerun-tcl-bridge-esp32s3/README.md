# Edgerun TCL Bridge ESP32-S3

Rust firmware for the JC3248W535 ESP32-S3 board used as a USB-attached debug bridge for TCL AC investigation.

The first bridge transport is USB Serial/JTAG on `/dev/ttyACM0`. This keeps the laptop on its normal Wi-Fi while the ESP32-S3 handles local Wi-Fi/BLE work near the AC.

## Build

```bash
cd devices/edgerun-tcl-bridge-esp32s3
. /path/to/export-esp.sh
cargo +esp check
cargo +esp build --release
```

## Flash

```bash
cd devices/edgerun-tcl-bridge-esp32s3
. /path/to/export-esp.sh
espflash flash --chip esp32s3 --port /dev/ttyACM0 --before usb-reset --monitor --non-interactive target/xtensa-esp32s3-none-elf/release/edgerun-tcl-bridge-esp32s3
```

If automatic reset cannot enter the ROM loader, put the board in download mode manually: hold `BOOT`, tap `RESET`, release `BOOT`, then rerun the same command. If that still resets out of bootloader, use `--before no-reset`.

## USB Commands

Commands are newline-delimited ASCII:

```text
help
ping
wifi-scan [max]
wifi-connect <ssid> [password]
wifi-status
wifi-stop
ap-start <ssid> [password] [channel]
hotspot-start <ssid> [password-or-dash] [channel]
hotspot-status
ble-scan
ble-provision
tcl-provision
```

The current firmware implements the USB shell, Wi-Fi scan/client/AP control, an ESP-hosted hotspot with DHCP, and TCL SoftAP provisioning over the ESP32-S3 network stack.

## ESP-Hosted Hotspot

To make the AC join the ESP32-S3 instead of making the laptop switch networks, start a hotspot whose SSID/password match the Wi-Fi credentials you provision into the AC:

```text
hotspot-start <ssid> <password-or-dash> [channel]
hotspot-status
```

The ESP uses `192.168.4.1/24` and runs a minimal DHCPv4 responder with one lease at `192.168.4.2`. `hotspot-status` reports whether the AP link is up and whether a DHCP client has taken the lease.

For open networks, pass `-` as the password:

```text
tcl-provision <ac-ap-ssid> <ac-ap-password-or-dash> <home-ssid> <home-password-or-dash> <bind-code> [device-ip]
```

Example when the AC SoftAP is open:

```text
tcl-provision TCL_AC_xxxx - MyHomeWifi my-home-password 123456 192.168.1.1
```

This connects the ESP32-S3 to the AC SoftAP, waits for DHCP, sends the APK-style UDP JSON payload to `device-ip:10000`, then tries the TCP XML fallback on the same port. BLE commands are still placeholders.
