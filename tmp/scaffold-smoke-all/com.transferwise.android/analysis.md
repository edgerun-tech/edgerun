# APK Behavior Analysis

## Identity

- Source package: `com.transferwise.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `8828`
- Static call sites: `518790`

## Event Model

- `ComWiseDeeplinkDeeplinkproxyactivity` routes to `com.wise.deeplink.DeepLinkProxyActivity` (activity)
  - Android action `android.intent.action.VIEW`
- `ComWiseNotificationsPresentationPreferencesNotificationpreferencesactivity` routes to `com.wise.notifications.presentation.preferences.NotificationPreferencesActivity` (activity)
  - Android action `android.intent.action.MAIN`

## Main Workflows

- `com.wise.deeplink.DeepLinkProxyActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=2, internal=7 signals=package_intents:2, activity_ui:1
  method `onResume`: platform=2, internal=7 signals=package_intents:2, activity_ui:1
  method `<init>`: platform=0, internal=6
  method `<clinit>`: platform=0, internal=1
  method `X0`: platform=0, internal=1
- `com.wise.notifications.presentation.preferences.NotificationPreferencesActivity` (activity) exported=true
  actions: `android.intent.action.MAIN`
  method `onCreate`: platform=0, internal=6
  method `h1`: platform=0, internal=25
  method `j1`: platform=0, internal=20
  method `f1`: platform=0, internal=14
  method `g1`: platform=0, internal=13

## Capability Evidence

- `accounts_identity`: 105 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 24831 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 10 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1424 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 235 call sites; contacts or calendar provider access
- `crypto_security`: 986 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 3106 call sites; local database or preference storage behavior
- `files_storage`: 4663 call sites; file, document, media store, or filesystem behavior
- `location`: 259 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 2781 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 249 call sites; notification posting, channels, or listener behavior
- `package_intents`: 10786 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 541 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 158 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 21372 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.wise.deeplink.DeepLinkProxyActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.wise.deeplink.DeepLinkProxyActivity` -> `onResume` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `drawable`(2097) `layout`(385)
- UI elements: `TextView`(318) `LinearLayout`(261)
- Literal UI text samples:
  - `Accept`
  - `Confirm Info`
- Asset samples:
  - `assets/PublicSuffixDatabase.list`
  - `assets/assets_3d/js/libs/DRACOLoader.js`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `aomedia.org`
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
