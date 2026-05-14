# APK Behavior Analysis

## Identity

- Source package: `com.whatsapp`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `10592`
- Static call sites: `715513`

## Event Model

- `ComWhatsappAccountsyncProfileactivity` routes to `com.whatsapp.accountsync.ProfileActivity` (activity)
  - Android action `android.intent.action.VIEW`
- `ComWhatsappAccountsyncCallcontactlandingactivity` routes to `com.whatsapp.accountsync.CallContactLandingActivity` (activity)
  - Android action `android.intent.action.VIEW`

## Main Workflows

- `com.whatsapp.accountsync.ProfileActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=2, internal=20 signals=activity_ui:2, package_intents:1
  method `onActivityResult`: platform=1, internal=5 signals=activity_ui:1
  method `A0W`: platform=5, internal=19 signals=activity_ui:3, package_intents:3
  method `<init>`: platform=0, internal=13
  method `A5C`: platform=0, internal=7
- `com.whatsapp.accountsync.CallContactLandingActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `<init>`: platform=0, internal=6
  method `A3a`: platform=1, internal=3 signals=activity_ui:1, package_intents:1
  method `AZZ`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 205 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 93967 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 134 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 2744 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 1036 call sites; contacts or calendar provider access
- `crypto_security`: 2910 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 18942 call sites; local database or preference storage behavior
- `files_storage`: 13334 call sites; file, document, media store, or filesystem behavior
- `location`: 632 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 6006 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 382 call sites; notification posting, channels, or listener behavior
- `package_intents`: 45835 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 817 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 323 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 25978 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `xml`(1)
- UI elements: `entry`(61) `language`(1)
- Asset samples:
  - `assets/ReferenceFaceShapeConstants/v01_high_end_face_compressed.bin`
  - `assets/aiv_spinner_animation.json`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `acs.whatsapp.com`
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
