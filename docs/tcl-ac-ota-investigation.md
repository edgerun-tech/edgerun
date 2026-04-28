# TCL AC OTA Investigation Notes

Status as of 2026-04-28.

## Local BLE State

The AC currently advertises as:

- Address: `3C:C5:DD:33:6D:1E`
- Name: `tcl_AC_t*ap_szkt`
- Service: `0000f100-0000-1000-8000-00805f9b34fb`

That is the legacy provisioning service. It is not advertising the OTA service found in the APK.

Live provisioning diagnostics on 2026-04-28:

- BLE connect works against `3C:C5:DD:33:6D:1E`.
- ATT MTU negotiation returns `247`.
- The AC uses the legacy `f100` JSON provisioning path, not the encrypted `f900` protocol.
- The provisioning write is accepted at the ATT layer.
- Without a real TCL bind-code response, the AC sends no BLE indication, does not answer UDP
  discovery, and remains on the `f100` pairing advertisement.

That means the immediate blocker is probably commissioning metadata (`bindCode`,
`deviceMqttEndpoint`, `deviceMqttEndpointV2`, and possibly tenant/product fields), not BLE socket
connectivity or ATT chunking.

## APK BLE Provisioning Path

For the observed `f100` service, TCL Home sends raw JSON over BLE:

- Service: `0000f100-0000-1000-8000-00805f9b34fb`
- Write characteristic: `0000f101-0000-1000-8000-00805f9b34fb`
- Indicate characteristic: `0000f102-0000-1000-8000-00805f9b34fb`
- CCCD: indication enable value `0x0002`

The APK code path is `com.tcl.libsoftap.action.NewConnectBleAction`. For legacy devices it uses
`ConfigReqFactory.buildUdpConfigReq(...)`, then writes the resulting JSON through
`BluetoothMirror.writeSplit(...)`. It requests a large MTU, then performs normal split writes; the
tested AC rejects ATT prepare-write with protocol error `0x10`.

Provisioning JSON shape:

```json
{
  "msgId": "100-999",
  "method": "setReq",
  "version": "1",
  "params": {
    "bindCode": "...",
    "ssid": "...",
    "password": "...",
    "timestamp": 1710000000,
    "timezone": 7,
    "timearea": "Asia/Bangkok",
    "serverPort": 443,
    "cloudType": "AWS",
    "caType": "release",
    "serverHost": "prod-center.aws.tcljd.com",
    "serverHostV2": "prod-center.aws.tcljd.com"
  }
}
```

The app treats BLE write success as one success signal, but it also has an ack path that parses
`params.mac` from an indication response. Our device has not produced that ack without real
commissioning data.

## LAN Discovery

After provisioning, TCL Home searches for the device locally:

- Listen UDP: `0.0.0.0:10074`
- Broadcast target: `255.255.255.255:10075`
- XML probe: `<searchDevice></searchDevice>`
- JSON probe: `{"msgId":"123","version":"123","method":"searchReq"}`

No LAN responses were observed after no-bind provisioning.

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

Provision from a credential file with SSID on line 1 and password on line 2:

```bash
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- connect 3C:C5:DD:33:6D:1E
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- provision-file ~/wifi.txt
```

Run the diagnostic flow, which connects, prints non-secret BLE/provisioning metadata, provisions,
runs LAN discovery, and prints `bluetoothctl info`:

```bash
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- diagnose-provision 3C:C5:DD:33:6D:1E ~/wifi.txt
```

If a TCL bind-code response has been captured to a JSON file, include it:

```bash
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- diagnose-provision 3C:C5:DD:33:6D:1E ~/wifi.txt /tmp/tcl-bind.json
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- provision-bind-file ~/wifi.txt /tmp/tcl-bind.json
```

Expected bind response fields:

- `bindCode`
- `deviceMqttEndpoint`
- `deviceMqttEndpointV2`

## Bind-Code API

The new TCL Home path requests a bind code with:

- Method: `POST`
- Path: `/v1/auth/get_bind_code`
- Body fields observed in APK code: `jobId`, `add_soure`, `entrance_id`, `productKey`,
  `cloudType`, `channelType`, optional `wifiMd5`

The request requires authenticated app headers. An unauthenticated request to
`https://prod-center.aws.tcljd.com/v1/auth/get_bind_code` returned HTTP 403.

Required header pattern from the APK:

- `ssoToken`: account token
- `accessToken`: IoT/SAAS token
- `appId: wx6e1af3fa84fbe523`
- `platform: android`
- `appVersion: 7.4.0`
- `tclhomeVersion: 6.1.1`
- `countryCode`, `timeZone`, `Accept-Language`
- `timestamp`, `nonce`
- `sign = md5(timestamp + nonce + accessToken)`

The CLI can log in with a TCL Home account and save the account/IoT token session locally:

```bash
# Prompts for the password without echoing it. Set TCL_PASSWORD only for non-interactive use.
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- login your-account@example.com
```

If the TCL Home account is backed by Google login, the APK uses Google Play Services to get a Google
`serverAuthCode` for client
`770138412962-r7k2eb426t7gmkk22rm79hhii2n639td.apps.googleusercontent.com`, then calls TCL
`/account/thirdParty/thirdLoginByAccessToken` with `platformId=6`. The CLI can perform the TCL-side
exchange if a valid server auth code is available:

```bash
cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- login-google-code "$TCL_GOOGLE_AUTH_CODE"
```

The session is written to `~/.config/edgerun/tcl-home-session.json` with mode `0600`. The login flow
matches the APK path:

- `POST /account/login?clientId=...` with `channel=app`, `username`, `password=md5(password)`,
  `captchaRule=2`, and basic OS/client metadata.
- For Google login, `POST /account/thirdParty/thirdLoginByAccessToken` with `platformId=6`,
  `token=<Google serverAuthCode>`, `flowTag=0`, `countryAbbr`, and the same app/device metadata.
- `POST /v3/global/cloud_url_get` with the SSO id/token to discover the IoT cloud URL.
- `POST /v3/auth/refresh_tokens` with `userId`, `ssoToken`, and the TCL Home app id to obtain the
  IoT/SAAS access token.

After login, only the product key is required for bind-code retrieval:

```bash
export TCL_PRODUCT_KEY=...

cargo run -p edgerun-tcl-ac-cli --features std --bin tcl-ac -- fetch-bind-code /tmp/tcl-bind.json
```

`fetch-bind-code` still accepts explicit token overrides for captured sessions:

```bash
export TCL_ACCESS_TOKEN=...
export TCL_SSO_TOKEN=...
export TCL_PRODUCT_KEY=...
```

Optional environment variables:

- `TCL_SESSION_FILE`, default `~/.config/edgerun/tcl-home-session.json`
- `TCL_ACCOUNT_HOST`, default selected from `TCL_COUNTRY_CODE`
- `TCL_CLIENT_ID`, default `54148614`
- `TCL_IOT_CENTER_URL`, default `https://prod-center.aws.tcljd.com`
- `TCL_IOT_BASE_URL`, default `https://prod-center.aws.tcljd.com`
- `TCL_ENTRANCE_ID`, default same as `TCL_PRODUCT_KEY`
- `TCL_JOB_ID`, default generated as `edgerun-<timestamp>`
- `TCL_ADD_SOURCE`, default `manual`
- `TCL_CLOUD_TYPE`, default `1`
- `TCL_CHANNEL_TYPE`, default `0`
- `TCL_WIFI_MD5`
- `TCL_COUNTRY_CODE`, default `TH`
- `TCL_TIME_ZONE`, default `Asia/Bangkok`
- `TCL_ACCEPT_LANGUAGE`, default `en-US`

The command uses a private temporary curl config instead of passing tokens on the command line, then
writes the response file with mode `0600`.

## Next Steps

1. Capture a real `/v1/auth/get_bind_code` response from the TCL app session or device logs.
2. Re-run `diagnose-provision` with the captured bind response JSON.
3. If the AC joins Wi-Fi, use UDP discovery to obtain local identity fields and then inspect local
   control ports.
4. Put the AC into firmware update mode only after normal local commissioning is understood.
5. If firmware must come from cloud, obtain the bound TCL `device_id` and an explicit user-approved
   auth session, then query `NewVersion.url`.
