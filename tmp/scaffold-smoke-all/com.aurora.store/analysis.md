# APK Behavior Analysis

## Identity

- Source package: `com.aurora.store`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `5910`
- Static call sites: `94548`

## Event Model

- `ComAuroraStoreMainactivity` routes to `com.aurora.store.MainActivity` (activity)
  - Android action `android.intent.action.MAIN`
  - Android action `android.intent.action.SEND`
  - Android action `android.intent.action.SHOW_APP_INFO`
  - Android action `android.intent.action.VIEW`
- `ComAuroraStoreDataReceiverDeviceownerreceiver` routes to `com.aurora.store.data.receiver.DeviceOwnerReceiver` (receiver)
  - Android action `android.app.action.DEVICE_ADMIN_ENABLED`

## Main Workflows

- `com.aurora.store.MainActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`, `android.intent.action.SEND`, `android.intent.action.SHOW_APP_INFO`, `android.intent.action.VIEW`
  method `onCreate`: platform=6, internal=40 signals=package_intents:2, activity_ui:1
  method `L`: platform=4, internal=9 signals=activity_ui:2
  method `<init>`: platform=3, internal=7
  method `K`: platform=4, internal=5 signals=activity_ui:2
  method `N`: platform=0, internal=1
- `com.aurora.store.data.receiver.DeviceOwnerReceiver` (receiver) exported=true
  actions: `android.app.action.DEVICE_ADMIN_ENABLED`
  method `<init>`: platform=1, internal=0 signals=activity_ui:1

## Capability Evidence

- `accounts_identity`: 20 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 14531 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 4 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 33 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 120 call sites; contacts or calendar provider access
- `crypto_security`: 68 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 685 call sites; local database or preference storage behavior
- `files_storage`: 1340 call sites; file, document, media store, or filesystem behavior
- `location`: 66 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 317 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 95 call sites; notification posting, channels, or listener behavior
- `package_intents`: 4389 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 1 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `work_background`: 2787 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.aurora.store.MainActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `color`(173) `color-v31`(21)
- Asset samples:
  - `assets/PublicSuffixDatabase.list`
  - `assets/dexopt/baseline.prof`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `android.clients.google.com`
- Native libraries:
  - `lib/arm64-v8a/libandroidx.graphics.path.so`
  - `lib/armeabi-v7a/libandroidx.graphics.path.so`

## Third-Party Modules

- `AndroidX Room database`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX WorkManager`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
