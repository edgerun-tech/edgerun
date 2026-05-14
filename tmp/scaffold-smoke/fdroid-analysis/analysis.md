# APK Behavior Analysis

## Identity

- Source package: `org.fdroid.fdroid`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `7770`
- Static call sites: `152529`

## Main Workflows

- `org.fdroid.fdroid.panic.PanicPreferencesActivity` (activity) exported=true
  actions: `info.guardianproject.panic.action.CONNECT`, `info.guardianproject.panic.action.DISCONNECT`
  method `onCreate`: platform=1, internal=8 signals=activity_ui:1
  method `<init>`: platform=0, internal=1
- `org.fdroid.fdroid.panic.PanicResponderActivity` (activity) exported=true
  actions: `info.guardianproject.panic.action.TRIGGER`
  method `onCreate`: platform=22, internal=22 signals=package_intents:4, activity_ui:3
  method `exitAndClear`: platform=1, internal=1 signals=activity_ui:1
  method `-$$Nest$mexitAndClear`: platform=0, internal=1
  method `<init>`: platform=0, internal=1
  method `resetRepos`: platform=0, internal=1
- `org.fdroid.fdroid.panic.CalculatorActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`
  method `onCreate`: platform=1, internal=7 signals=activity_ui:1
  method `op`: platform=18, internal=5 signals=activity_ui:7
  method `eval`: platform=15, internal=4 signals=activity_ui:2
  method `number`: platform=7, internal=2 signals=activity_ui:3
  method `c`: platform=6, internal=0 signals=activity_ui:3
- `org.fdroid.fdroid.views.repos.AddRepoActivity` (activity) exported=true
  actions: `android.intent.action.SEND`, `android.intent.action.VIEW`
  method `onCreate`: platform=4, internal=11 signals=package_intents:3, activity_ui:1, work_background:1
  method `onResume`: platform=0, internal=3
  method `fetchIfRepoUri`: platform=13, internal=18 signals=activity_ui:3
  method `onFetchRepo`: platform=5, internal=10 signals=network_web:2, package_intents:2
  method `onCreate$lambda$2`: platform=6, internal=3 signals=package_intents:3
- `org.fdroid.fdroid.views.AppDetailsActivity` (activity) exported=true
  actions: `android.intent.action.SHOW_APP_INFO`
  method `onCreate`: platform=7, internal=38 signals=activity_ui:6, package_intents:1
  method `onStart`: platform=0, internal=10
  method `onActivityResult`: platform=2, internal=4 signals=package_intents:2
  method `onOptionsItemSelected`: platform=36, internal=7 signals=package_intents:15, activity_ui:13, network_web:1
  method `updateAppStatus`: platform=10, internal=14 signals=activity_ui:2, package_intents:1
- `org.fdroid.fdroid.views.main.MainActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`, `android.intent.action.SEARCH`, `android.intent.action.VIEW`
  method `onCreate`: platform=7, internal=28 signals=activity_ui:6, package_intents:2
  method `onRequestPermissionsResult`: platform=4, internal=4 signals=activity_ui:2, package_intents:2
  method `onResume`: platform=0, internal=4
  method `onNewIntent`: platform=0, internal=3
  method `onStart`: platform=0, internal=3
- `org.fdroid.fdroid.nearby.UsbDeviceAttachedReceiver` (receiver) exported=false
  actions: `android.hardware.usb.action.USB_DEVICE_ATTACHED`
  method `onReceive`: platform=23, internal=1 signals=package_intents:6, work_background:1
  method `<init>`: platform=1, internal=0 signals=package_intents:1
- `org.fdroid.fdroid.nearby.UsbDeviceDetachedReceiver` (receiver) exported=false
  actions: `android.hardware.usb.action.USB_DEVICE_DETACHED`
  method `onReceive`: platform=20, internal=1 signals=package_intents:4
  method `<clinit>`: platform=1, internal=0
  method `<init>`: platform=1, internal=0 signals=package_intents:1

## Capability Evidence

- `accounts_identity`: 39 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 12496 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 54 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 150 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 103 call sites; contacts or calendar provider access
- `crypto_security`: 4661 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 2053 call sites; local database or preference storage behavior
- `files_storage`: 4260 call sites; file, document, media store, or filesystem behavior
- `location`: 56 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 1042 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 118 call sites; notification posting, channels, or listener behavior
- `package_intents`: 5133 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 76 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 8 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 2628 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `color`(185) `color-v31`(32) `color-v23`(9) `color-night-v8`(3)
- Asset samples:
  - `assets/dexopt/baseline.prof`
  - `assets/dexopt/baseline.profm`
  - `assets/index.template.html`
  - `assets/logback.xml`
  - `assets/swap-icon.png`
  - `assets/swap-icon.svg`
  - `assets/swap-tick-done.png`
  - `assets/swap-tick-not-done.png`

## Network And Native Surface

- Static domain strings:
  - `4everland.io`
  - `apt.izzysoft.de`
  - `archive.newpipe.net`
  - `briarproject.org`
  - `developer.android.com`
  - `en.wikipedia.org`
  - `example.com`
  - `example.org`
- Native libraries:
  - `lib/arm64-v8a/libandroidx.graphics.path.so`
  - `lib/armeabi-v7a/libandroidx.graphics.path.so`
  - `lib/x86/libandroidx.graphics.path.so`
  - `lib/x86_64/libandroidx.graphics.path.so`

## Third-Party Modules

- `AndroidX Room database`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX WorkManager`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `Bouncy Castle crypto`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `OkHttp`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
