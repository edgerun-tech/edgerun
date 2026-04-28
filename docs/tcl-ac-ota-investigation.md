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

The TCL Home APK has two OTA paths:

- BLE/local OTA, mostly used by soundbar code paths, downloads a file from `NewVersion.url`.
- Cloud/device OTA, used by this AC, exposes firmware version and job state to the app but does not
  return a package URL in the read-only app API response.

AWS/cloud endpoints:

- `GET /v3/ota/version/get?device_id=<device_id>`
- `GET /v3/ota/device/lastest?device_id=<device_id>`

EMQ/overseas endpoints:

- `GET /v1/ota/version/get?device_id=<device_id>&cloud_type=<cloud_type>`
- `GET /v1/ota/device/lastest?device_id=<device_id>&cloud_type=<cloud_type>`

The production AWS center host observed in the APK is:

- `https://prod-center.aws.tcljd.com`

The bound account session resolved the AC's regional IoT API host to:

- `https://prod-sgp.aws.tcljd.com`

For the bound AC:

- Device ID: `DSxvOivgAAE`
- Product key: `V6VHeDcmZcw78ioZ`
- Current firmware: `V8-R82CT04-LF1V025.0.1.11`
- Available firmware: `V8-R82CT04-LF1V206.180.3.52`
- Protocol type: `1`
- Latest observed OTA job: `TIMED_OUT`

The authenticated `GET /v3/ota/version/get?device_id=DSxvOivgAAE` response includes
`newVersion.version`, `newVersion.versionName`, `upgradeType`, and release notes, but no
`newVersion.url`. The authenticated `GET /v3/ota/device/lastest?device_id=DSxvOivgAAE` response
includes job/status fields, but no package URL. Read-only probes of likely package/info endpoints
also returned either the same version metadata or AWS `MissingAuthenticationToken`, which indicates
an undeployed API Gateway route for those paths rather than a usable firmware endpoint.

Do not call `POST /v3/ota/user/accept` just to look for the blob: in the APK this is the user
"update now" action and may enqueue a real OTA job for the appliance. If that path is tested, it
should be treated as a mutating operation and followed by explicit job-status and cancel handling.

On 2026-04-28 the accept path was tested after explicit operator approval. It returned success with
the known OTA job id, but the app REST status still only exposed job state and did not return a
package URL. The useful firmware path was AWS IoT, not the TCL REST OTA API.

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

## AWS IoT OTA Blob Retrieval

The AC OTA job is an AWS IoT Job that points to an AWS IoT Stream. Direct S3 access is denied to the
app's Cognito role, but AWS IoT MQTT-based file delivery can read the same object in blocks.

Fresh AWS session discovery:

- `GET https://prod-sgp.aws.tcljd.com/v1/auth/service/loadBalance`
- Required headers for this `@NoIotToken` endpoint are the app `ssoToken` and
  `appId: wx6e1af3fa84fbe523`.
- The response includes `cognitoId`, `cognitoToken`, `mqttEndpoint`, `saasToken`, and `userId`.
- The returned `cognitoToken` must be exchanged with Cognito using login provider
  `cognito-identity.amazonaws.com`. Passing it under `tcl_account_dev` fails with
  `Invalid login token. Can't pass in a Cognito token.`

Observed AWS values:

- Region: `ap-southeast-1`
- IoT endpoint: `a2qjkbbsk6qn2u-ats.iot.ap-southeast-1.amazonaws.com`
- Identity pool: `ap-southeast-1:3141be4f-75b0-4bb7-9728-fbaced243dbe`
- Thing name: `DSxvOivgAAE`
- Job id: `V6VHeDcmZcw78ioZ_1773384355228_cota`

The AWS IoT job document:

```json
{
  "checksum": "63a75f9ed1579c1468673af4f293ec0b",
  "command": "cota",
  "fileId": 29,
  "fileSize": 847738,
  "imageVer": "V8-R82CT04-LF1V206.180.3.52",
  "isForce": 0,
  "otaType": "local",
  "streamId": "V6VHeDcmZcw78ioZ_V8-R82CT04-LF1V206180352_1773384355228_cota",
  "versionName": "system"
}
```

`DescribeStream` resolves the stream file to:

- Bucket: `420520409389-ap-southeast-1-backend-prod`
- Key:
  `ota-overseas-iot/V6VHeDcmZcw78ioZ/V8-R82CT04-LF1V206.180.3.52/18d6d09116f64d4c8d60cc16221cf725/system.bin`
- File id: `29`

The app Cognito role cannot `s3:GetObject` that key directly. Downloading succeeds through AWS IoT
MQTT-based file delivery:

- Subscribe: `$aws/things/DSxvOivgAAE/streams/<streamId>/data/json`
- Subscribe: `$aws/things/DSxvOivgAAE/streams/<streamId>/rejected/json`
- Publish block requests: `$aws/things/DSxvOivgAAE/streams/<streamId>/get/json`

Example block request:

```json
{
  "c": "er-0",
  "f": 29,
  "l": 131072,
  "o": 0,
  "n": 1
}
```

The response contains `p` as base64 payload. Blocks are indexed by `i`. The full blob was retrieved
as seven blocks.

Retrieved firmware:

- Path: `/tmp/tcl-firmware/mqtt-stream/system.bin`
- Size: `847738`
- MD5: `63a75f9ed1579c1468673af4f293ec0b`
- SHA-256: `c05c752eb511cb1d6ab6ab94faab5f5b708b820c2d74e48c5aec71eca4705fb4`

Initial binary notes:

- Header starts with `THOS`.
- Version strings include `V8-R82CT04-LF1V206.180.3.52`.
- Platform strings include `AmebaZII`, `RT8720CF`, and `AmebaZIIRTL8710C`.
- The image contains TCL/AWS OTA, MQTT, TLS, Wi-Fi, BLE, LAN, and local-control related strings.
- It is not a mountable archive by `file` or `7z`; treat it as a Realtek Ameba firmware image until
  a specific unpacker is identified.

## Firmware Container Analysis

The outer OTA blob is a TCL `THOS` container around a Realtek/AmebaZII firmware payload.

Top-level wrapper:

- `0x00000000`: `THOS`
- wrapper type/value bytes: `64 00`
- version string length: `0x001b`
- version string: `V8-R82CT04-LF1V206.180.3.52`
- repeated version string: `V8-R82CT04-LF1V206.180.3.52`
- `0x00000040`: little-endian payload length `0x000cee7a` (`847482` bytes)
- following ASCII MD5: `5f1fd606f315dc90fc561adeaefd1237`

That MD5 is the digest of `system.bin[0x100..]`, not the whole OTA file. The whole-file MD5 is the
AWS IoT job checksum. This gives two validation layers:

- AWS IoT/job checksum over the full `system.bin`.
- TCL wrapper checksum over the payload starting at `0x100`.

Observed hashes:

- `md5(system.bin) = 63a75f9ed1579c1468673af4f293ec0b`
- `md5(system.bin[0x100..]) = 5f1fd606f315dc90fc561adeaefd1237`

The payload starts at `0x100` with additional non-archive metadata and a nested `THOS` marker at
`0x10d`. A plausible Cortex-M style vector-table candidate appears at file offset `0x1b8`:

- stack pointer candidate: `0x2000004c`
- reset vector candidate: `0x080c007f`

However, direct Thumb disassembly from that mapping does not produce clean code, and the standard
`ltchiptool` AmebaZ2 parser does not accept the blob at offsets `0`, `0x100`, `0x10d`, `0x1b0`,
`0x1b8`, `0x200`, `0x400`, or `0x1000`. The failures are structural: the parser interprets bytes as
huge section sizes rather than valid AmebaZ2 image headers. Treat the current file as a TCL-packaged
or otherwise transformed AmebaZII image, not a raw LibreTiny/Realtek SDK OTA image.

Current extraction status:

- `binwalk` only detects AES S-box and SHA-256 constants, not a filesystem or archive.
- `7z` does not recognize it as a compressed archive.
- No Realtek partition-table calibration pattern was found.
- Entropy is high across most of the blob, with a much more ASCII-heavy region around
  `0x0b0000..0x0c0000`, where most cloud/control strings are visible.
- `ltchiptool` confirms the target family is supported as Realtek AmebaZ2/RTL8720C, but this TCL
  OTA container needs a custom unpacking step before normal image parsing works.

## Firmware Cloud, OTA, BLE, and LAN Clues

The recovered firmware contains strings for the same flows observed dynamically in the APK and AWS
IoT account:

- AWS OTA/MQTT: `epi_aws_ota_mqtt_cli1`, `aws_ota`, `MQTT GET OTA JOB INFO`,
  `AWS_OTA_STATUS`, `mqtt.publish`, `mqtt_recv`, `ota/notify`, `min/ota/job`.
- Thing authorization/profile: `/v2/auth/thing_re`, `authorize/join`, `CLOUD_PROFILE`,
  `dBalance`, `token`, `MQTTURL`, `MQTTEP`.
- Shadow/TSL: `shadow/u`, `tcl_tsl`, `TSL_TABLE`, `tsl`, `TSL ERROR`, `cur tsl`.
- LAN/local: `TCP_IP`, `UDP_RAW`, `udp peek`, `lan_send`, `socket fail`, `_mcu_local_`,
  `HTTP`, `WEBSOCKET`.
- BLE/provisioning/OTA: `AUTH_PASSKE`, `BLE_SCAN_DATA_UP`, `BLE IF SEND`,
  `EN_LM_BLE_OTA`, `BLE_OTA_AVAILABLE`, `BLE_OTA_GET_VER`, `ota_ble_deinit`.
- AC controls: `work_mode`, `WORK_MODE`, `temperature`, `windScan`, `eco`, `turbo`,
  `beep`, `display`, `ElecStatus`, `filter`.

The important conclusion is that cloud, OTA, BLE, LAN, and the AC TSL/control model are all present
in the device firmware. The slow cloud behavior is not caused by the app simply relaying commands to
a dumb module; the module itself runs the AWS IoT/MQTT/shadow client and has local network code.

## Cloud Shadow Control Model

The live AWS IoT shadow for `DSxvOivgAAE` exposes the AC command/state schema. Desired and reported
state use the same field names, which means cloud control is a shadow desired-state update and the
device reports convergence back through AWS IoT.

Core controls:

- `powerSwitch`
- `workMode`
- `targetTemperature`
- `windSpeed`
- `ECO`
- `turbo`
- `screen`
- `beepSwitch`
- `sleep`
- `horizontalSwitch`
- `verticalSwitch`
- `horizontalWind`
- `verticalWind`
- `verticalDirection`
- `horizontalDirection`
- `currentTemperature`
- `errorCode`

Additional feature flags observed:

- `PTC`
- `3DAirSurply`
- `antiMoldew`
- `healthy`
- `selfLearn`
- `infrDirect`
- `temperatureType`
- `acType`
- `eightAddHot`
- `highTemperatureWind`
- `feelTheWind`
- `LRWideAngleFreeze`
- `advancedECOMode`
- `silenceSwitch`
- `generatorMode`
- `lightSense`
- `filterBlockStatus`
- `filterBlockSwitch`
- `selfClean`
- `softWind`
- `targetElectric`
- `constantTemp`
- `rightHorizontalDirection`
- `rightHorizontalSwitch`
- `rightWindSpeed`
- `temperatureHumidityControlling`
- `constTemperatureDehumidification`
- `aiFunction`
- `rightTurbo`
- `rightSilenceSwitch`
- `infrPower`
- `twoTemperature`
- `polarityFreeze`
- `surroundWind`
- `twoTemperatureSkew`

The same semantic fields appear, partially mangled by binary/string interleaving, in the firmware
strings. That gives a concrete field list to use when implementing LAN/local control once the local
transport is identified.

## Custom Firmware Feasibility

Custom firmware is not ruled out, but the OTA route is not yet a straightforward flashing path.

What looks feasible:

- The hardware family is Realtek AmebaZII/RTL8720C/RTL8720CF-class, for which UART flashing tools
  and SDK-style image tooling exist.
- The OTA blob contains a normal-looking TCL wrapper checksum and exposed platform strings, so it is
  not opaque end-to-end encryption.
- Physical access to the module UART/boot pins may allow full flash read/write independent of the
  TCL cloud OTA path, subject to secure-boot/efuse state.

What blocks OTA-based custom firmware today:

- The app REST OTA API exposes job/version state, not a generic firmware-upload endpoint.
- AWS IoT Streams allows block download of TCL's object but does not provide write access to install
  arbitrary objects.
- The retrieved `system.bin` is not accepted by the standard AmebaZ2 parser as a raw OTA image.
- Realtek AmebaZ2 supports secure boot/root-of-trust features, and the firmware contains certificate
  and crypto material. Without checking the module's efuse/secure-boot state or reverse-engineering
  the TCL image transform/signature, arbitrary OTA replacement should be assumed rejected.

Practical paths from here:

1. Prefer local control first: finish commissioning, then exercise UDP/TCP/HTTP/WebSocket discovery
   and map the `_mcu_local_`/TSL local transport.
2. If local control remains locked to cloud auth, open the unit and identify the Wi-Fi module pins,
   UART boot mode, flash size, and secure-boot state. Dump flash before writing anything.
3. Build a TCL `THOS` unpacker: validate the `0x100` payload MD5, decode the nested payload layout,
   then retry AmebaZ2 parsing and function/string cross-reference work on the decoded image.

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

1. Analyze `/tmp/tcl-firmware/mqtt-stream/system.bin` as a Realtek AmebaZII/RTL8720 firmware image.
2. Search the firmware for local-control protocol strings, BLE service handling, LAN discovery, and
   UART/control framing.
3. Capture a real `/v1/auth/get_bind_code` response from the TCL app session or device logs.
4. Re-run `diagnose-provision` with the captured bind response JSON.
5. If the AC joins Wi-Fi, use UDP discovery to obtain local identity fields and then inspect local
   control ports.
