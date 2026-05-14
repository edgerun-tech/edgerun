# APK Behavior Analysis

## Identity

- Source package: `com.android.vending`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `12063`
- Static call sites: `444635`

## Event Model

- No typed workflow events were generated.

## Main Workflows

- No manifest entrypoint classes had directly parsed code.

## Capability Evidence

- `accounts_identity`: 238 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 29814 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 6 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 919 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 223 call sites; contacts or calendar provider access
- `crypto_security`: 1993 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 2226 call sites; local database or preference storage behavior
- `files_storage`: 6637 call sites; file, document, media store, or filesystem behavior
- `location`: 147 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 4658 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 233 call sites; notification posting, channels, or listener behavior
- `package_intents`: 21913 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 77 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 157 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 20738 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- No generated workflow method currently records capability intents.

## UI And Resource Hints

- Resource buckets: `color`(313) `color-v31`(44)
- UI elements: `entry`(74) `language`(1)
- Asset samples:
  - `assets/ProductSans-Regular.ttf`
  - `assets/apploading/generic_category.json`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `admob-gmats.uc.r.appspot.com`
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
