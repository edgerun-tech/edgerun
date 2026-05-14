# APK Behavior Analysis

## Identity

- Source package: `io.homeassistant.companion.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `12852`
- Static call sites: `307502`

## Event Model

- `IoHomeassistantCompanionAndroidMatterMattercommissioningactivity` routes to `io.homeassistant.companion.android.matter.MatterCommissioningActivity` (activity)
  - Android action `com.google.android.gms.home.matter.ACTION_COMMISSION_DEVICE`
- `IoHomeassistantCompanionAndroidLaunchLaunchactivity` routes to `io.homeassistant.companion.android.launch.LaunchActivity` (activity)
  - Android action `android.intent.action.MAIN`

## Main Workflows

- `io.homeassistant.companion.android.matter.MatterCommissioningActivity` (activity) exported=true
  actions: `com.google.android.gms.home.matter.ACTION_COMMISSION_DEVICE`
  method `onResume`: platform=12, internal=21 signals=package_intents:1
  method `onCreate`: platform=0, internal=5
  method `onNewIntent`: platform=0, internal=3
  method `onCreate$lambda$0$0`: platform=0, internal=42
  method `<init>`: platform=0, internal=10
- `io.homeassistant.companion.android.launch.LaunchActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`
  method `onCreate`: platform=0, internal=8
  method `onCreate$lambda$1$0`: platform=0, internal=54
  method `onCreate$lambda$1`: platform=0, internal=10
  method `<init>`: platform=0, internal=7
  method `viewModel_delegate$lambda$0$0`: platform=0, internal=4

## Capability Evidence

- `accounts_identity`: 60 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 14719 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 260 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1215 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 418 call sites; contacts or calendar provider access
- `crypto_security`: 293 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 1035 call sites; local database or preference storage behavior
- `files_storage`: 5179 call sites; file, document, media store, or filesystem behavior
- `location`: 406 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 1682 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 457 call sites; notification posting, channels, or listener behavior
- `package_intents`: 9204 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 181 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 39 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 12487 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `io.homeassistant.companion.android.matter.MatterCommissioningActivity` -> `onResume` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `layout`(607) `drawable`(327)
- UI elements: `include`(1234) `ViewStub`(1058)
- Literal UI text samples:
  - `Cancel`
  - `Description (Required)`
- Asset samples:
  - `assets/PublicSuffixDatabase.list`
  - `assets/dexopt/baseline.prof`

## Network And Native Surface

- Static domain strings:
  - `altbeacon.github.io`
  - `aomedia.org`
- Native libraries: none found

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
