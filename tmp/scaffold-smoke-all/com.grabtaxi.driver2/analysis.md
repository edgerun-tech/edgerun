# APK Behavior Analysis

## Identity

- Source package: `com.grabtaxi.driver2`
- Confidence: `medium_static_needs_dynamic_trace`
- Unique API signatures: `14615`
- Static call sites: `938299`

## Event Model

- `ComGrabPartnerSdkWrapperDeeplinkDeeplinkactivity` routes to `com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity` (activity)
  - Android action `android.intent.action.VIEW`
- `ComGrabGkycSdkFeaturesBasicUiBasicactivity` routes to `com.grab.gkyc.sdk.features.basic.ui.BasicActivity` (activity)

## Main Workflows

- `com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity` (activity) exported=true
  actions: `android.intent.action.VIEW`
  method `onCreate`: platform=1, internal=3 signals=activity_ui:1, package_intents:1
  method `onNewIntent`: platform=0, internal=3
  method `getRedirectUrl`: platform=3, internal=3 signals=package_intents:2, activity_ui:1
  method `launchChromeManagerActivity`: platform=0, internal=5
  method `<init>`: platform=0, internal=1
- `com.grab.gkyc.sdk.features.basic.ui.BasicActivity` (activity) exported=true
  method `G2`: platform=1, internal=40
  method `L2`: platform=0, internal=8
  method `<init>`: platform=0, internal=1
  method `I2`: platform=0, internal=1
  method `N2`: platform=0, internal=1

## Capability Evidence

- `accounts_identity`: 133 call sites; accounts, credentials, login, or authentication APIs
- `activity_ui`: 52998 call sites; Android UI, activity, dialog, window, or view behavior
- `bluetooth_nearby`: 428 call sites; Bluetooth or nearby-device discovery/connect behavior
- `camera_media_capture`: 2886 call sites; camera, microphone, recorder, or media capture behavior
- `contacts_calendar`: 1311 call sites; contacts or calendar provider access
- `crypto_security`: 6054 call sites; key, certificate, cipher, signature, or keystore behavior
- `database_preferences`: 6754 call sites; local database or preference storage behavior
- `files_storage`: 13658 call sites; file, document, media store, or filesystem behavior
- `location`: 1312 call sites; location, GNSS/GPS, geocoder, or map-position behavior
- `network_web`: 5047 call sites; network, socket, HTTP, TLS, or WebView behavior
- `notifications`: 457 call sites; notification posting, channels, or listener behavior
- `package_intents`: 19216 call sites; intent, package manager, broadcast, or cross-app behavior
- `sensors`: 875 call sites; sensors, haptics, NFC, fingerprint, or biometric behavior
- `sms_telephony`: 164 call sites; SMS, MMS, phone, carrier, or telephony behavior
- `work_background`: 31845 call sites; jobs, alarms, wake locks, services, or background work

## Generated Capability Intent Plan

- `com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity` -> `onCreate` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence
- `com.grab.partner.sdk.wrapper.deeplink.DeepLinkActivity` -> `getRedirectUrl` records:
  - `app_events.send_or_receive_app_event` from `package_intents` evidence

## UI And Resource Hints

- Resource buckets: `drawable`(3611) `layout`(2196)
- UI elements: `TextView`(4919) `ImageView`(1657)
- Literal UI text samples:
  - `300 m`
  - `APPLY`
- Asset samples:
  - `assets/LICENSE`
  - `assets/base_hms_app_root.cer`

## Network And Native Surface

- Static domain strings:
  - `accounts.google.com`
  - `another-example.com`
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
