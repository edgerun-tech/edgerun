# APK Behavior Analysis

## Identity

- Source package: `com.facebook.orca`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `10992`
- Static call sites: `733887`

## Event Model

- `ComFacebookOrcaThreadviewThreadviewbubblesactivity` routes to `com.facebook.orca.threadview.ThreadViewBubblesActivity` (activity)
  - Android action `com.facebook.orca.THREAD_VIEW_BUBBLE`

## Main Workflows

- `com.facebook.orca.threadview.ThreadViewBubblesActivity` (activity) exported=false
  actions: `com.facebook.orca.THREAD_VIEW_BUBBLE`
  method `Cih`: platform=1, internal=29 signals=package_intents:1
  method `A31`: platform=0, internal=14
  method `<init>`: platform=0, internal=7
  method `A3G`: platform=1, internal=6
  method `A2z`: platform=0, internal=3

## Capability Evidence

- `accounts_identity`: 380 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 59110 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 141 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 3272 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 1201 call sites; contacts or calendar provider access
- `crypto_security`: 1240 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 5563 call sites; local database or preference storage behavior
- `files_storage`: 11684 call sites; file, document, media store, or filesystem behavior
- `location`: 641 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 9034 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 795 call sites; notification posting, channels, or listener behavior
- `package_intents`: 31499 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 644 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 482 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 53940 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.facebook.orca.threadview.ThreadViewBubblesActivity` -> `Cih` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `xml`(1)
- UI elements: `entry`(89) `language`(1)
- Asset samples:
  - `assets/ContactOmnistoreSchema.fbs`
  - `assets/ContactOmnistoreSchema.idna`

## Network And Native Surface

- Static domain strings:
  - `0.freebasics.com`
  - `about.fb.com`
- Native libraries:
  - `lib/arm64-v8a/libandroidx.graphics.path.so`
  - `lib/arm64-v8a/libappcomponentfactory-jni.so`

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
