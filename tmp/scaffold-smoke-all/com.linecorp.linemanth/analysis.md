# APK Behavior Analysis

## Identity

- Source package: `com.linecorp.linemanth`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `8972`
- Static call sites: `448367`

## Event Model

- `ComLinemanMartFeatureTelemedFeatureVideocallTelemedvideocallactivity` routes to `com.lineman.mart.feature.telemed.feature.videocall.TelemedVideoCallActivity` (activity)
- `ComLinecorpLinemanthAndroidFeatureVoipPresentationVoipactivity` routes to `com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity` (activity)

## Main Workflows

- `com.lineman.mart.feature.telemed.feature.videocall.TelemedVideoCallActivity` (activity) exported=true
  method `onCreate`: platform=7, internal=10 signals=activity_ui:7, package_intents:1
  method `onStart`: platform=4, internal=1 signals=activity_ui:4
  method `onNewIntent`: platform=0, internal=3
  method `s`: platform=1, internal=5 signals=package_intents:1
  method `onStop`: platform=4, internal=1 signals=activity_ui:4
- `com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity` (activity) exported=true
  method `onNewIntent`: platform=0, internal=5
  method `onStart`: platform=4, internal=1 signals=activity_ui:4
  method `onCreate`: platform=1, internal=3 signals=activity_ui:1, package_intents:1
  method `H`: platform=2, internal=15 signals=package_intents:2
  method `onStop`: platform=4, internal=1 signals=activity_ui:4

## Capability Evidence

- `accounts_identity`: 75 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 35669 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 33 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1909 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 342 call sites; contacts or calendar provider access
- `crypto_security`: 1487 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 3388 call sites; local database or preference storage behavior
- `files_storage`: 6266 call sites; file, document, media store, or filesystem behavior
- `location`: 428 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 4919 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 311 call sites; notification posting, channels, or listener behavior
- `package_intents`: 15269 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 273 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 118 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 27521 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.lineman.mart.feature.telemed.feature.videocall.TelemedVideoCallActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.lineman.mart.feature.telemed.feature.videocall.TelemedVideoCallActivity` -> `s` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.linecorp.linemanth.android.feature.voip.presentation.VoipActivity` -> `H` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `drawable`(1356) `layout`(1219)
- UI elements: `androidx.constraintlayout.widget.ConstraintLayout`(932) `ImageView`(873)
- Literal UI text samples:
  - `Apply Coupon`
  - `Can you read Thai?`
- Asset samples:
  - `assets/2797639a44ed74080b2bd9fbcbb2c8421c485bd2f5ba41fa8adc182e5d5b8872/3930f676-7171-6c18-a267-01e3af3b904c`
  - `assets/braze-html-bridge.js`

## Network And Native Surface

- Static domain strings:
  - `access.line.me`
  - `accounts.google.com`
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
