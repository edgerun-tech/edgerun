# APK Behavior Analysis

## Identity

- Source package: `com.google.android.googlequicksearchbox`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `15278`
- Static call sites: `852693`

## Event Model

- `ComGoogleAndroidGooglequicksearchboxSearchwidgetprovider` routes to `com.google.android.googlequicksearchbox.SearchWidgetProvider` (receiver)
  - Android action `android.appwidget.action.APPWIDGET_UPDATE`
  - Android action `android.appwidget.action.APPWIDGET_UPDATE_OPTIONS`
  - Android action `com.google.android.finsky.intent.action.UPDATE_DSE`
  - Android action `com.google.android.finsky.intent.action.UPDATE_DSE_APP_STATE`

## Main Workflows

- `com.google.android.googlequicksearchbox.SearchWidgetProvider` (receiver) exported=true
  actions: `android.appwidget.action.APPWIDGET_UPDATE`, `android.appwidget.action.APPWIDGET_UPDATE_OPTIONS`, `com.google.android.finsky.intent.action.UPDATE_DSE`, `com.google.android.finsky.intent.action.UPDATE_DSE_APP_STATE`
  method `onReceive`: platform=1, internal=6
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 305 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 63592 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 404 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 2965 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 401 call sites; contacts or calendar provider access
- `crypto_security`: 1530 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 2950 call sites; local database or preference storage behavior
- `files_storage`: 6316 call sites; file, document, media store, or filesystem behavior
- `location`: 650 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 8246 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 479 call sites; notification posting, channels, or listener behavior
- `package_intents`: 34079 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 534 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 144 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 27571 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `color`(471) `color-v31`(96)
- UI elements: `entry`(79) `language`(1)
- Asset samples:
  - `assets/4_device_looking_fail_360.json`
  - `assets/4_device_looking_in_360.json`

## Network And Native Surface

- Static domain strings:
  - `accountlinking-pa.clients6.google.com`
  - `accountlinking-pa.googleapis.com`
- Native libraries:
  - `lib/x86_64/libagsa_renderer_jni.so`
  - `lib/x86_64/libandroidx.graphics.path.so`

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
