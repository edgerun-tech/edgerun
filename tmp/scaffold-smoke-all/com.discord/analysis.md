# APK Behavior Analysis

## Identity

- Source package: `com.discord`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `7987`
- Static call sites: `213048`

## Event Model

- `ComDiscordShareShareactivity` routes to `com.discord.share.ShareActivity` (activity)
  - Android action `android.intent.action.SEND`
  - Android action `android.intent.action.SEND_MULTIPLE`
- `ComDiscordMainMainactivity` routes to `com.discord.main.MainActivity` (activity)
  - Android action `android.intent.action.VIEW`
  - Android action `com.discord.intent.action.CONNECT`
  - Android action `com.discord.intent.action.SDK`

## Main Workflows

- `com.discord.share.ShareActivity` (activity) exported=true
  actions: `android.intent.action.SEND`, `android.intent.action.SEND_MULTIPLE`
  method `<init>`: platform=0, internal=1
  method `getActivityDelegate`: platform=0, internal=1
- `com.discord.main.MainActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`, `com.discord.intent.action.CONNECT`, `com.discord.intent.action.SDK`
  method `onPictureInPictureModeChanged`: platform=0, internal=7
  method `onStop`: platform=0, internal=3
  method `onUserLeaveHint`: platform=0, internal=3
  method `currentReactContext`: platform=0, internal=2
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 66 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 20342 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 20 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 1353 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 212 call sites; contacts or calendar provider access
- `crypto_security`: 232 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 1432 call sites; local database or preference storage behavior
- `files_storage`: 4662 call sites; file, document, media store, or filesystem behavior
- `location`: 92 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 1802 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 311 call sites; notification posting, channels, or listener behavior
- `package_intents`: 6842 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 208 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 17 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 5800 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `raw`(1291) `drawable`(270)
- UI elements: `TextView`(263) `LinearLayout`(155)
- Literal UI text samples:
  - `Animations FPS Summaries`
  - `Cancel`
- Asset samples:
  - `assets/com/appsflyer/internal/1ef4f10b9adfdad9a-`
  - `assets/com/appsflyer/internal/4c337fab34977e37d-`

## Network And Native Surface

- Static domain strings:
  - `account.samsung.cn`
  - `account.samsung.com`
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
