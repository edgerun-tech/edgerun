# APK Behavior Analysis

## Identity

- Source package: `com.tuya.smart`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `13338`
- Static call sites: `995601`

## Event Model

- `ComThingclipsSocialAmazonActivityTriplealexaaccountlinkactivity` routes to `com.thingclips.social.amazon.activity.TripleAlexaAccountLinkActivity` (activity)
  - Android action `android.intent.action.VIEW`
- `ComThingclipsSocialAmazonActivityAlexaauthactivity` routes to `com.thingclips.social.amazon.activity.AlexaAuthActivity` (activity)
  - Android action `android.intent.action.VIEW`

## Main Workflows

- `com.thingclips.social.amazon.activity.TripleAlexaAccountLinkActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=1, internal=3 signals=activity_ui:1, package_intents:1
  method `onNewIntent`: platform=0, internal=2
  method `Wc`: platform=1, internal=3
  method `<init>`: platform=0, internal=1
- `com.thingclips.social.amazon.activity.AlexaAuthActivity` (activity) exported=unknown
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=4, internal=4 signals=activity_ui:1, package_intents:1
  method `onNewIntent`: platform=0, internal=2
  method `Wc`: platform=1, internal=2
  method `Xc`: platform=1, internal=2
  method `Yc`: platform=0, internal=3

## Capability Evidence

- `accounts_identity`: 176 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 82137 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 765 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 4488 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 1313 call sites; contacts or calendar provider access
- `crypto_security`: 3196 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 6610 call sites; local database or preference storage behavior
- `files_storage`: 19315 call sites; file, document, media store, or filesystem behavior
- `location`: 711 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 7124 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 516 call sites; notification posting, channels, or listener behavior
- `package_intents`: 32086 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 549 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 71 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 33946 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.thingclips.social.amazon.activity.TripleAlexaAccountLinkActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.thingclips.social.amazon.activity.AlexaAuthActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `layout`(2700) `drawable`(1989)
- UI elements: `TextView`(4223) `LinearLayout`(2464)
- Literal UI text samples:
  - `0s`
  - `1x`
- Asset samples:
  - `assets/Manrope-Bold.ttf`
  - `assets/Manrope-Medium.ttf`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `admob-gmats.uc.r.appspot.com`
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
