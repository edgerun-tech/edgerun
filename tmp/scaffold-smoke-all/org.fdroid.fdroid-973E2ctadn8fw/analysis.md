# APK Behavior Analysis

## Identity

- Source package: `org.fdroid.fdroid`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `7770`
- Static call sites: `152529`

## Event Model

- `OrgFdroidFdroidPanicPanicpreferencesactivity` routes to `org.fdroid.fdroid.panic.PanicPreferencesActivity` (activity)
  - Android action `info.guardianproject.panic.action.CONNECT`
  - Android action `info.guardianproject.panic.action.DISCONNECT`
- `OrgFdroidFdroidPanicPanicresponderactivity` routes to `org.fdroid.fdroid.panic.PanicResponderActivity` (activity)
  - Android action `info.guardianproject.panic.action.TRIGGER`

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

## Generated Capability Intent Plan

- `org.fdroid.fdroid.panic.PanicResponderActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `color`(185) `color-v31`(32)
- Asset samples:
  - `assets/dexopt/baseline.prof`
  - `assets/dexopt/baseline.profm`

## Network And Native Surface

- Static domain strings:
  - `4everland.io`
  - `apt.izzysoft.de`
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
