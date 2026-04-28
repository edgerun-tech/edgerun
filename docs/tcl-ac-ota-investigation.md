# TCL AC OTA Investigation Notes

Status as of 2026-04-28.

## Local BLE State

The AC currently advertises as:

- Address: `3C:C5:DD:33:6D:1E`
- Name: `tcl_AC_t*ap_szkt`
- Service: `0000f100-0000-1000-8000-00805f9b34fb`

That is the legacy provisioning service. It is not advertising the OTA service found in the APK.

## APK OTA Paths

The TCL Home APK obtains firmware metadata from TCL cloud APIs. The firmware URL is returned in
`NewVersion.url`.

AWS/cloud endpoints:

- `GET /v3/ota/version/get?device_id=<device_id>`
- `GET /v3/ota/device/lastest?device_id=<device_id>`

EMQ/overseas endpoints:

- `GET /v1/ota/version/get?device_id=<device_id>&cloud_type=<cloud_type>`
- `GET /v1/ota/device/lastest?device_id=<device_id>&cloud_type=<cloud_type>`

The production AWS base host observed in the APK is:

- `https://prod-center.aws.tcljd.com`

Unauthenticated requests return AWS `MissingAuthenticationToken`, so fetching firmware through this
path requires a bound TCL account session and the actual cloud `device_id`.

Required app headers include:

- `ssoToken`
- `accessToken`
- `appId: wx6e1af3fa84fbe523`
- `platform: android`
- `appVersion: 7.4.0`
- `THomeVersion: 6.1.1`
- `countryCode`
- `timeZone`
- `Accept-Language`
- `timestamp`
- `nonce`
- `sign = md5(timestamp + nonce + accessToken)`

## APK BLE OTA Protocol

The APK contains a BLE OTA path aimed at a soundbar-style transport:

- OTA service: `e49a25f8-f69a-11e8-8eb2-f2801f1b9fd1`
- Write characteristic: `e49a25e0-f69a-11e8-8eb2-f2801f1b9fd1`
- Notify characteristic: `e49a28e1-f69a-11e8-8eb2-f2801f1b9fd1`

The OTA command frame starts with:

- byte 0: server id, usually `0x09`
- byte 1: command id
- bytes 2..: TLV block

Observed command IDs:

- `0x01`: check remote status / upgrade request
- `0x02`: negotiation request/response
- `0x03`: data transfer request
- `0x06`: validate package
- `0x09`: ack / app ready

Do not send OTA frames until the AC is confirmed to expose this OTA service. The normal AC
provisioning service is different.

## Local Tooling

Use the TCL CLI scan and inspect commands:

```bash
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- scan 10
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- inspect <address>
```

`inspect` is non-destructive: it connects and dumps the GATT service/characteristic database.

## Next Steps

1. Put the AC into firmware update mode.
2. Run `scan` and identify any changed address/name/service UUIDs.
3. Run `inspect` on the update-mode address.
4. If it exposes `e49a25f8-f69a-11e8-8eb2-f2801f1b9fd1`, implement a read-only OTA handshake probe.
5. If firmware must come from cloud, obtain the bound TCL `device_id` and an explicit user-approved
   auth session, then query `NewVersion.url`.
