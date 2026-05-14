# APK Behavior Analysis

## Identity

- Source package: `asuk.com.android.app`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `11752`
- Static call sites: `733373`

## Event Model

- `ComOnesignalNotificationopenedactivityhms` routes to `com.onesignal.NotificationOpenedActivityHMS` (activity)
  - Android action `android.intent.action.VIEW`
- `ComOnesignalNotificationopenedreceiver` routes to `com.onesignal.NotificationOpenedReceiver` (activity)

## Main Workflows

- `com.onesignal.NotificationOpenedActivityHMS` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=1, internal=1 signals=activity_ui:1
  method `onNewIntent`: platform=1, internal=1 signals=activity_ui:1, package_intents:1
  method `a`: platform=2, internal=1 signals=activity_ui:2, package_intents:1
  method `<init>`: platform=1, internal=0 signals=activity_ui:1
  method `b`: platform=0, internal=1
- `com.onesignal.NotificationOpenedReceiver` (activity) exported=true
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 143 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 59402 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 25 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 3761 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 551 call sites; contacts or calendar provider access
- `crypto_security`: 2486 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 5456 call sites; local database or preference storage behavior
- `files_storage`: 9492 call sites; file, document, media store, or filesystem behavior
- `location`: 930 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 5074 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 468 call sites; notification posting, channels, or listener behavior
- `package_intents`: 22029 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 758 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 493 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 37988 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.onesignal.NotificationOpenedActivityHMS` -> `onNewIntent` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.onesignal.NotificationOpenedActivityHMS` -> `a` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `drawable`(1950) `layout`(1730)
- UI elements: `androidx.appcompat.widget.AppCompatTextView`(1717) `androidx.constraintlayout.widget.ConstraintLayout`(1477)
- Literal UI text samples:
  - `ArMzA`
  - `Clear`
- Asset samples:
  - `assets/300529d92b526da61c63b319f40ae2800a0ad9286b7128c33af24559907ff367/851c24e1-41cb-c6b6-f5db-f434cc5bdc46`
  - `assets/PublicSuffixDatabase.list`

## Network And Native Surface

- Static domain strings:
  - `7eleven-streamer.trueid-preprod.net`
  - `7eleven-streamer.trueid.net`
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
