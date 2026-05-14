# APK Behavior Analysis

## Identity

- Source package: `com.google.android.apps.messaging`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `13665`
- Static call sites: `608739`

## Event Model

- `ComGoogleAndroidAppsMessagingUiConversationLaunchconversationactivity` routes to `com.google.android.apps.messaging.ui.conversation.LaunchConversationActivity` (activity)
  - Android action `android.intent.action.SENDTO`
  - Android action `android.intent.action.VIEW`
- `ComGoogleAndroidAppsMessagingMainMainactivity` routes to `com.google.android.apps.messaging.main.MainActivity` (activity)

## Main Workflows

- `com.google.android.apps.messaging.ui.conversation.LaunchConversationActivity` (activity) exported=true
  actions: `android.intent.action.SENDTO`, `android.intent.action.VIEW`
  method `onCreate`: platform=1, internal=30
  method `onActivityResult`: platform=1, internal=4
  method `onNewIntent`: platform=1, internal=4
  method `onRequestPermissionsResult`: platform=1, internal=4
  method `onResume`: platform=1, internal=4
- `com.google.android.apps.messaging.main.MainActivity` (activity) exported=true
  method `onResume`: platform=3, internal=37 signals=package_intents:1
  method `onNewIntent`: platform=10, internal=26 signals=package_intents:6
  method `onActivityResult`: platform=5, internal=22
  method `onCreate`: platform=5, internal=19 signals=activity_ui:2
  method `onStart`: platform=1, internal=13

## Capability Evidence

- `accounts_identity`: 245 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 23905 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 4 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1627 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 444 call sites; contacts or calendar provider access
- `crypto_security`: 2361 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 3629 call sites; local database or preference storage behavior
- `files_storage`: 7946 call sites; file, document, media store, or filesystem behavior
- `location`: 223 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 5554 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 330 call sites; notification posting, channels, or listener behavior
- `package_intents`: 20317 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 299 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 337 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 26860 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.google.android.apps.messaging.main.MainActivity` -> `onResume` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.google.android.apps.messaging.main.MainActivity` -> `onNewIntent` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `color`(287) `color-v31`(58)
- UI elements: `entry`(80) `language`(1)
- Asset samples:
  - `assets/AltFormats_255`
  - `assets/AltFormats_27`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `ad.doubleclick.net`
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
