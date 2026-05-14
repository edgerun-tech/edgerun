# APK Behavior Analysis

## Identity

- Source package: `com.instagram.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `11413`
- Static call sites: `1172453`

## Event Model

- `ComInstagramMainactivityLauncheractivity` routes to `com.instagram.mainactivity.LauncherActivity` (activity)
- `ComInstagramMainactivityInstagrammainactivity` routes to `com.instagram.mainactivity.InstagramMainActivity` (activity)

## Main Workflows

- `com.instagram.mainactivity.LauncherActivity` (activity) exported=true
  method `onCreate`: platform=28, internal=100 signals=package_intents:12, activity_ui:6
  method `onResume`: platform=9, internal=32
  method `onStart`: platform=9, internal=32
  method `onDestroy`: platform=9, internal=32
  method `onPause`: platform=9, internal=32
- `com.instagram.mainactivity.InstagramMainActivity` (activity) exported=true
  method `onNewIntent`: platform=15, internal=44 signals=package_intents:7, network_web:3, activity_ui:2
  method `A0h`: platform=75, internal=343 signals=package_intents:36, activity_ui:14, work_background:2
  method `A0i`: platform=71, internal=327 signals=package_intents:39, activity_ui:15, work_background:2
  method `A1p`: platform=68, internal=151 signals=package_intents:47, activity_ui:3, work_background:3
  method `A21`: platform=20, internal=156 signals=package_intents:4, activity_ui:3

## Capability Evidence

- `accounts_identity`: 225 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 110017 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 152 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 4646 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 1485 call sites; contacts or calendar provider access
- `crypto_security`: 1547 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 3043 call sites; local database or preference storage behavior
- `files_storage`: 15171 call sites; file, document, media store, or filesystem behavior
- `location`: 1107 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 8766 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 560 call sites; notification posting, channels, or listener behavior
- `package_intents`: 49266 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 718 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 490 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 60252 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.instagram.mainactivity.LauncherActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.instagram.mainactivity.InstagramMainActivity` -> `onNewIntent` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `network.request_network_or_webview` from `network_web` evidence
- `com.instagram.mainactivity.InstagramMainActivity` -> `A0h` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
  - `network.request_network_or_webview` from `network_web` evidence
- `com.instagram.mainactivity.InstagramMainActivity` -> `A0i` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
  - `network.request_network_or_webview` from `network_web` evidence
- `com.instagram.mainactivity.InstagramMainActivity` -> `A1p` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
  - `background_tasks.schedule_or_handle_background_work` from `work_background` evidence
- `com.instagram.mainactivity.InstagramMainActivity` -> `A21` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `xml`(1)
- UI elements: `entry`(90) `language`(1)
- Asset samples:
  - `assets/1b042b2402db51831ac2b0e996d109b38b2b22b72f140a414e9caa24bd5028c0_bundled_payload.json`
  - `assets/2594bb736e28570f2dc1bc56511a4f62871c38d36e6742e69c45393f451aeed7_bundled_payload.json`

## Network And Native Surface

- Static domain strings:
  - `about.fb.com`
  - `about.meta.com`
- Native libraries:
  - `lib/x86_64/libandroidx.graphics.path.so`
  - `lib/x86_64/libarcore_sdk_c.so`

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
