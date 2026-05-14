# APK Behavior Analysis

## Identity

- Source package: `com.google.android.webview`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `4878`
- Static call sites: `44132`

## Event Model

- No typed workflow events were generated.

## Main Workflows

- No manifest entrypoint classes had directly parsed code.

## Capability Evidence

- `accounts_identity`: 58 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 5667 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 101 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 479 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 106 call sites; contacts or calendar provider access
- `crypto_security`: 111 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 421 call sites; local database or preference storage behavior
- `files_storage`: 1189 call sites; file, document, media store, or filesystem behavior
- `location`: 98 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 773 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 89 call sites; notification posting, channels, or listener behavior
- `package_intents`: 2624 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 238 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 4 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 4316 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `color`(8) `color-v23`(6)
- UI elements: `entry`(74) `language`(1)
- Asset samples:
  - `assets/chrome_100_percent.pak+com.google.android.webview+`
  - `assets/chrome_200_percent.pak+com.google.android.webview+`

## Network And Native Surface

- Static domain strings:
  - `android.com`
  - `bugs.chromium.org`
- Native libraries:
  - `lib/x86/libcrashpad_handler_trampoline.so`
  - `lib/x86/libwebviewchromium.so`

## Third-Party Modules

- `Chromium/WebView code`: default=`replace_gradually`, breakage=`high`; runtime or rendering framework; rebuild screen-by-screen instead of silently dropping it
- `Google Play libraries`: default=`preserve_if_reachable`, breakage=`unknown`; module appears in the app; dynamic trace should decide whether it is required

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
