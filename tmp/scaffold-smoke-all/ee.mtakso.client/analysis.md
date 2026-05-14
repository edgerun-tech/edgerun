# APK Behavior Analysis

## Identity

- Source package: `ee.mtakso.client`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `11601`
- Static call sites: `563230`

## Event Model

- `EeMtaksoClientNewbaseRidehailingmapactivity` routes to `ee.mtakso.client.newbase.RideHailingMapActivity` (activity)
- `EeMtaksoClientNewbaseVoipVoiptrampolineactivity` routes to `ee.mtakso.client.newbase.voip.VoipTrampolineActivity` (activity)

## Main Workflows

- `ee.mtakso.client.newbase.RideHailingMapActivity` (activity) exported=true
  method `onCreate`: platform=6, internal=30 signals=work_background:3, package_intents:2, activity_ui:1
  method `onActivityResult`: platform=0, internal=21
  method `onNewIntent`: platform=2, internal=5 signals=package_intents:2, activity_ui:1
  method `onStart`: platform=0, internal=3
  method `createRouter`: platform=0, internal=11
- `ee.mtakso.client.newbase.voip.VoipTrampolineActivity` (activity) exported=true
  method `onCreate`: platform=15, internal=14 signals=package_intents:8, activity_ui:6, work_background:4
  method `o`: platform=0, internal=6
  method `<init>`: platform=0, internal=3
  method `<clinit>`: platform=0, internal=1
  method `n`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 200 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 36499 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 327 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1102 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 474 call sites; contacts or calendar provider access
- `crypto_security`: 817 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 4116 call sites; local database or preference storage behavior
- `files_storage`: 7844 call sites; file, document, media store, or filesystem behavior
- `location`: 413 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 3376 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 338 call sites; notification posting, channels, or listener behavior
- `package_intents`: 12025 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 368 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 53 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 13245 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `ee.mtakso.client.newbase.RideHailingMapActivity` -> `onCreate` records:
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `ee.mtakso.client.newbase.RideHailingMapActivity` -> `onNewIntent` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `ee.mtakso.client.newbase.voip.VoipTrampolineActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence

## UI And Resource Hints

- Resource buckets: `drawable`(1483) `layout`(1046)
- UI elements: `eu.bolt.uikit.components.text.BoltTextView`(947) `LinearLayout`(641)
- Literal UI text samples:
  - `Cancel`
  - `Default`
- Asset samples:
  - `assets/braze-html-in-app-message-bridge.js`
  - `assets/dexopt/baseline.prof`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `admin-panel.bolt.eu`
- Native libraries: none found

## Third-Party Modules

- `AndroidX Camera`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX Room database`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
