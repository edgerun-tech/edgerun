# APK Behavior Analysis

## Identity

- Source package: `com.lazada.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `11086`
- Static call sites: `560477`

## Event Model

- `ComLazadaActivitiesEnteractivity` routes to `com.lazada.activities.EnterActivity` (activity)
  - Android action `android.intent.action.MAIN`
  - Android action `android.intent.action.VIEW`
  - Android action `com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION`
- `ComLazadaAndroidVideoproductionBizPlayerVideoplayeractivity` routes to `com.lazada.android.videoproduction.biz.player.VideoPlayerActivity` (activity)
  - Android action `com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION`

## Main Workflows

- `com.lazada.activities.EnterActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`, `android.intent.action.VIEW`, `com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION`
  method `onCreate`: platform=31, internal=45 signals=activity_ui:15, package_intents:14
  method `onActivityResult`: platform=5, internal=8
  method `onResume`: platform=0, internal=12
  method `onRequestPermissionsResult`: platform=2, internal=5
  method `onStart`: platform=0, internal=6
- `com.lazada.android.videoproduction.biz.player.VideoPlayerActivity` (activity) exported=true
  actions: `com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION`
  method `onCreate`: platform=13, internal=8 signals=activity_ui:6, network_web:3, package_intents:2
  method `onResume`: platform=0, internal=2
  method `initView`: platform=8, internal=18 signals=activity_ui:5
  method `start`: platform=0, internal=16
  method `startForResult`: platform=0, internal=8

## Capability Evidence

- `accounts_identity`: 148 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 75460 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 128 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1462 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 574 call sites; contacts or calendar provider access
- `crypto_security`: 1067 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 6884 call sites; local database or preference storage behavior
- `files_storage`: 10605 call sites; file, document, media store, or filesystem behavior
- `location`: 358 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 9804 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 430 call sites; notification posting, channels, or listener behavior
- `package_intents`: 24962 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 846 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 131 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 24903 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.lazada.activities.EnterActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.lazada.android.videoproduction.biz.player.VideoPlayerActivity` -> `onCreate` records:
  - `network.request_network_or_webview` from `network_web` evidence
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `a5`(2078) `n`(1477)
- UI elements: `LinearLayout`(87) `TextView`(56)
- Asset samples:
  - `assets/EuclidCircularA-Bold.otf`
  - `assets/EuclidCircularA-Medium.otf`

## Network And Native Surface

- Static domain strings:
  - `access.line.me`
  - `accounts.google.com`
- Native libraries: none found

## Third-Party Modules

- `AndroidX Room database`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `Firebase`: default=`preserve_minimal`, breakage=`medium`; often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
