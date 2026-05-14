# APK Behavior Analysis

## Identity

- Source package: `com.facebook.katana`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `3498`
- Static call sites: `42069`

## Event Model

- No typed workflow events were generated.

## Main Workflows

- No manifest entrypoint classes had directly parsed code.

## Capability Evidence

- `accounts_identity`: 26 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 1945 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 3 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 38 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 2 call sites; contacts or calendar provider access
- `crypto_security`: 101 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 123 call sites; local database or preference storage behavior
- `files_storage`: 3582 call sites; file, document, media store, or filesystem behavior
- `location`: 6 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 375 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 214 call sites; notification posting, channels, or listener behavior
- `package_intents`: 1821 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 18 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 4 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 2150 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `xml`(1)
- UI elements: `entry`(90) `language`(1)
- Asset samples:
  - `assets/1b042b2402db51831ac2b0e996d109b38b2b22b72f140a414e9caa24bd5028c0_bundled_payload.json`
  - `assets/3405970032f1116f474bfd5786a228dc3ec2b12d65954e29c69e52cac7ca6c8b_bundled_payload.json`

## Network And Native Surface

- Static domain strings:
  - `b-www.facebook.com`
  - `fburl.com`
- Native libraries:
  - `lib/x86_64/libachilles-jni.so`
  - `lib/x86_64/libandroidx.graphics.path.so`

## Third-Party Modules

- `Meta/Facebook SDK`: default=`preserve_if_login_or_share_required`, breakage=`medium`; may provide login, sharing, attribution, or web redirect handling; make optional unless a workflow uses it

## Runtime Observation Needed

- capture launch-to-first-use screen flow and selected UI actions
- identify camera/media entrypoints and whether they are core workflow or vendor SDK
- identify exact user action that requests location
- record actual contacted domains and authentication redirects
- record which services/receivers fire during normal use
- toggle third-party modules and observe breakage before default opt-out
