# APK Behavior Analysis

## Identity

- Source package: `com.smart_id`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `9071`
- Static call sites: `281393`

## Event Model

- `ComStagnationlabSkMainactivity` routes to `com.stagnationlab.sk.MainActivity` (activity)
  - Android action `android.intent.action.MAIN`
  - Android action `android.intent.action.VIEW`
- `ComHuaweiHmsFlutterPushHmsFlutterhmsmessageservice` routes to `com.huawei.hms.flutter.push.hms.FlutterHmsMessageService` (service)
  - Android action `com.huawei.push.action.MESSAGING_EVENT`

## Main Workflows

- `com.stagnationlab.sk.MainActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`, `android.intent.action.VIEW`
  method `onCreate`: platform=819, internal=285 signals=activity_ui:116, package_intents:84, work_background:32
  method `onNewIntent`: platform=8, internal=16 signals=package_intents:4
  method `onActivityResult`: platform=2, internal=5
  method `onResume`: platform=0, internal=5
  method `onStart`: platform=0, internal=1
- `com.huawei.hms.flutter.push.hms.FlutterHmsMessageService` (service) exported=false
  actions: `com.huawei.push.action.MESSAGING_EVENT`
  method `onMessageReceived`: platform=7, internal=12 signals=package_intents:5
  method `onCreate`: platform=0, internal=1
  method `onTokenError`: platform=16, internal=31 signals=package_intents:3, activity_ui:2
  method `onMessageDelivered`: platform=15, internal=11
  method `onNewToken`: platform=5, internal=17

## Capability Evidence

- `accounts_identity`: 51 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 18912 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 10 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 961 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 290 call sites; contacts or calendar provider access
- `crypto_security`: 7707 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 1018 call sites; local database or preference storage behavior
- `files_storage`: 7411 call sites; file, document, media store, or filesystem behavior
- `location`: 186 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 1762 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 272 call sites; notification posting, channels, or listener behavior
- `package_intents`: 8422 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 457 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 128 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 8534 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.stagnationlab.sk.MainActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
  - `camera_media.capture_or_process_media` from `camera_media_capture` evidence
  - `location.request_location` from `location` evidence
  - `sms_telephony.use_sms_or_phone_state` from `sms_telephony` evidence
- `com.stagnationlab.sk.MainActivity` -> `onNewIntent` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.huawei.hms.flutter.push.hms.FlutterHmsMessageService` -> `onMessageReceived` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.huawei.hms.flutter.push.hms.FlutterHmsMessageService` -> `onTokenError` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Asset samples:
  - `assets/com/iproov/sdk/core/do/5c350702d62af8cca-`
  - `assets/com/iproov/sdk/core/do/82764fad400a232eb-`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `clientonly.readid.com`
- Native libraries:
  - `lib/x86_64/liba947af.so`
  - `lib/x86_64/libapp.so`

## Third-Party Modules

- `AndroidX Camera`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required
- `AndroidX WorkManager`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
