# APK Behavior Analysis

## Identity

- Source package: `jp.naver.line.android`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `10537`
- Static call sites: `1140159`

## Event Model

- `ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityBasic1splashactivity` routes to `com.linecorp.line.settings.customappicon.CustomAppIconSplashActivity$Basic1SplashActivity` (activity)
  - Android action `android.intent.action.MAIN`
- `ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityPromotionhalloween1splashactivity` routes to `com.linecorp.line.settings.customappicon.CustomAppIconSplashActivity$PromotionHalloween1SplashActivity` (activity)
  - Android action `android.intent.action.MAIN`

## Main Workflows

- `com.linecorp.line.settings.customappicon.CustomAppIconSplashActivity$Basic1SplashActivity` (activity) exported=false
  actions: `android.intent.action.MAIN`
  method `<init>`: platform=0, internal=1
- `com.linecorp.line.settings.customappicon.CustomAppIconSplashActivity$PromotionHalloween1SplashActivity` (activity) exported=false
  actions: `android.intent.action.MAIN`
  method `<init>`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 173 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 90402 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 256 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 3497 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 1063 call sites; contacts or calendar provider access
- `crypto_security`: 2462 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 6784 call sites; local database or preference storage behavior
- `files_storage`: 11195 call sites; file, document, media store, or filesystem behavior
- `location`: 509 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 9027 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 290 call sites; notification posting, channels, or listener behavior
- `package_intents`: 36237 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 986 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 112 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 30930 call sites; jobs, alarms, wake locks, services, or background work

## UI And Resource Hints

- Resource buckets: `drawable`(4672) `layout`(3491)
- UI elements: `TextView`(4621) `ImageView`(3291)
- Literal UI text samples:
  - `1x`
  - `2x`
- Asset samples:
  - `assets/PublicSuffixDatabase.list`
  - `assets/SenseID_Liveness_Silent.lic`

## Network And Native Surface

- Static domain strings:
  - `1176-ti.cloud.v-key.com`
  - `1176-tla.cloud.v-key.com`
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
